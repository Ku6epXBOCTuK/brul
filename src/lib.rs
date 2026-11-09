//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use brul_codegen::runtime::accepts_prop;
use brul_gui::{Widget, WidgetEvent, WidgetId, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
use brul_script::eval;
use brul_syntax::ast::Expr;

pub use brul_codegen::runtime::{
    Binding, BindingTarget, Built, ComponentEvent, ComponentHandle, Handler, HandlerEvent,
    Instance, PropArgument, PropBinding, Rebuilt, RowInfo, Structure, View, instance,
};
pub use brul_gui::window::WindowConfig;
pub use brul_script::{Component, Error, Host, Value, Variable, execute_program};
pub use brul_syntax::ast;
pub use brul_syntax::markup::PropType;
pub use brul_syntax::script;

pub struct Ui {
    pub component: ComponentHandle,
    pub tree: Rc<RefCell<WidgetTree>>,
    handlers: Vec<Handler>,
    window_handlers: Vec<Handler>,
    commands: Rc<RefCell<Vec<window::WindowCommand>>>,
    bindings: Vec<Binding>,
    props: Vec<PropBinding>,
    structures: Vec<Structure>,
    rows: Vec<(WidgetId, Vec<RowInfo>)>,
    requests: Rc<RefCell<Vec<WidgetId>>>,
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    theme: Option<String>,
    scope: EffectScope,
}

struct CommandHost {
    commands: Rc<RefCell<Vec<window::WindowCommand>>>,
    app: Option<Box<dyn Host>>,
}

impl Host for CommandHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        if name == "set_theme" {
            if arguments.len() != 1 {
                return Err(Error::Arity {
                    name: name.to_owned(),
                    expected: "one",
                });
            }
            let Value::String(theme) = &arguments[0] else {
                return Err(Error::Argument {
                    name: "set_theme",
                    expected: "String",
                    found: arguments[0].type_name(),
                });
            };
            self.commands
                .borrow_mut()
                .push(window::WindowCommand::SetTheme(theme.clone()));
            return Ok(Value::Null);
        }
        let command = match name {
            "exit" => window::WindowCommand::Exit,
            "dragWindow" => window::WindowCommand::Drag,
            _ => return self.delegate(name, arguments),
        };
        if !arguments.is_empty() {
            return Err(Error::Arity {
                name: name.to_owned(),
                expected: "no",
            });
        }
        self.commands.borrow_mut().push(command);
        Ok(Value::Null)
    }

    fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        match &mut self.app {
            Some(host) => host.every(interval_ms, function),
            None => Err(Error::Unsupported("every")),
        }
    }
}

impl CommandHost {
    fn delegate(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        match &mut self.app {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }
}

impl Ui {
    fn assemble(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
        theme: Option<String>,
        host: Option<Box<dyn Host>>,
    ) -> Self {
        let Built {
            widget,
            bindings,
            handlers,
            props,
            structures,
        } = built;
        let commands = Rc::new(RefCell::new(Vec::new()));
        for (interval_ms, function) in component.borrow_mut().take_every() {
            if interval_ms.is_finite() && interval_ms > 0.0 {
                commands.borrow_mut().push(window::WindowCommand::Every {
                    interval: Duration::from_secs_f64(interval_ms / 1000.0),
                    function,
                });
            } else {
                eprintln!("every interval must be a positive number");
            }
        }
        component.borrow_mut().set_host(CommandHost {
            commands: Rc::clone(&commands),
            app: host,
        });
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        let window_handlers = handlers
            .iter()
            .filter(|handler| handler.widget.is_none())
            .cloned()
            .collect();
        let ui = Self {
            component,
            tree,
            handlers,
            window_handlers,
            commands,
            bindings,
            props,
            structures,
            rows: Vec::new(),
            requests: Rc::new(RefCell::new(Vec::new())),
            view,
            theme,
            scope: EffectScope::new(),
        };
        ui.bind(&ui.bindings);
        ui.bind_props(&ui.props);
        ui.bind_structures(&ui.structures);
        ui
    }

    fn bind(&self, bindings: &[Binding]) {
        for binding in bindings {
            let binding = binding.clone();
            let tree = Rc::clone(&self.tree);
            self.scope.run(|| {
                Effect::new(move || apply(&binding, &binding.component, &tree));
            });
        }
    }

    fn bind_props(&self, props: &[PropBinding]) {
        for prop in props {
            let prop = prop.clone();
            self.scope.run(|| {
                Effect::new(move || apply_prop(&prop));
            });
        }
    }

    fn bind_structures(&self, structures: &[Structure]) {
        for structure in structures {
            let structure = structure.clone();
            let component = Rc::clone(&self.component);
            let requests = Rc::clone(&self.requests);
            let initialized = Rc::new(Cell::new(false));
            let last: Rc<RefCell<Option<Value>>> = Rc::new(RefCell::new(None));
            self.scope.run(|| {
                Effect::new(move || {
                    component.borrow().track();
                    let value = (structure.value)(&component);
                    if !initialized.replace(true) {
                        *last.borrow_mut() = value;
                        return;
                    }
                    if *last.borrow() == value {
                        return;
                    }
                    *last.borrow_mut() = value;
                    let mut requests = requests.borrow_mut();
                    if !requests.contains(&structure.parent) {
                        requests.push(structure.parent);
                    }
                });
            });
        }
    }

    fn rebuild(&mut self) -> Result<(), brul_syntax::error::Error> {
        let built = (self.view)(self.theme.as_deref(), &self.component).map_err(|message| {
            brul_syntax::error::Error {
                message,
                line: 1,
                column: 1,
            }
        })?;
        self.tree.borrow_mut().replace_root(built.widget);
        let mut handlers = built.handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.handlers = handlers;
        self.bindings = built.bindings;
        self.props = built.props;
        self.structures = built.structures;
        self.rows.clear();
        self.requests.borrow_mut().clear();
        self.scope = EffectScope::new();
        self.bind(&self.bindings);
        self.bind_props(&self.props);
        self.bind_structures(&self.structures);
        Ok(())
    }

    fn set_theme(&mut self, theme: &str) -> Result<(), brul_syntax::error::Error> {
        if self.theme.as_deref() == Some(theme) {
            return Ok(());
        }
        let previous = self.theme.replace(theme.to_owned());
        if let Err(error) = self.rebuild() {
            self.theme = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn dispatch(&mut self, events: &[WidgetEvent]) -> Vec<window::WindowCommand> {
        for event in events {
            if let WidgetEvent::Timer(function) = event {
                if let Err(error) = self
                    .component
                    .borrow_mut()
                    .call_function(function, Vec::new())
                {
                    eprintln!("{error}");
                }
                continue;
            }
            let Some((widget, target)) = handler_target(event) else {
                continue;
            };
            let Some(handler) = self
                .handlers
                .iter()
                .find(|handler| handler.widget == widget && handler.event == target)
            else {
                continue;
            };
            let function = handler.function.clone();
            let component = Rc::clone(&handler.component);
            let arguments = match event {
                WidgetEvent::Key(code) => vec![Value::Object(vec![(
                    "code".to_owned(),
                    Value::String(code.clone()),
                )])],
                _ => Vec::new(),
            };
            if let Err(error) = component.borrow_mut().call_function(&function, arguments) {
                eprintln!("{error}");
            }
        }
        let commands = std::mem::take(&mut *self.commands.borrow_mut());
        let mut output = Vec::new();
        for command in commands {
            match command {
                window::WindowCommand::SetTheme(theme) => {
                    if let Err(error) = self.set_theme(&theme) {
                        eprintln!("{error}");
                    }
                }
                command => output.push(command),
            }
        }
        output
    }

    pub fn settle(&mut self) {
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        if requests.is_empty() {
            return;
        }
        let mut structures = std::mem::take(&mut self.structures);
        let mut rows = std::mem::take(&mut self.rows);
        let mut bindings = Vec::new();
        let mut handlers = Vec::new();
        let mut props = Vec::new();
        let mut changed = false;
        for parent in requests {
            let Some(index) = structures
                .iter()
                .position(|structure| structure.parent == parent)
            else {
                continue;
            };
            if !self.tree.borrow().contains(parent) {
                continue;
            }
            let structure = structures[index].clone();
            let (built, rebuilt_rows) =
                match (structure.rebuild)(self.theme.as_deref(), &self.component) {
                    Ok(result) => result,
                    Err(message) => {
                        eprintln!("{message}");
                        continue;
                    }
                };
            let Built {
                widget,
                bindings: added_bindings,
                handlers: added_handlers,
                structures: added_structures,
                props: added_props,
            } = built;
            let children = if structure.each {
                let old_rows = rows
                    .iter()
                    .position(|(id, _)| *id == parent)
                    .map(|position| rows.remove(position).1)
                    .unwrap_or_default();
                rows.push((parent, rebuilt_rows.clone()));
                reconcile_each(&self.tree, parent, &old_rows, widget, &rebuilt_rows)
            } else {
                widget.into_children()
            };
            self.tree
                .borrow_mut()
                .replace_all_children(parent, children);
            bindings.extend(added_bindings);
            handlers.extend(added_handlers);
            props.extend(added_props);
            structures.extend(added_structures);
            changed = true;
        }
        self.structures = structures;
        self.rows = rows;
        if !changed {
            return;
        }
        let tree = Rc::clone(&self.tree);
        self.structures
            .retain(|structure| tree.borrow().contains(structure.parent));
        self.rows.retain(|(id, _)| tree.borrow().contains(*id));
        self.bindings.extend(bindings);
        self.bindings
            .retain(|binding| tree.borrow().contains(binding.widget));
        self.props.extend(props);
        self.props
            .retain(|prop| tree.borrow().contains(prop.widget));
        self.handlers.extend(handlers);
        self.handlers.retain(|handler| match handler.widget {
            Some(widget) => tree.borrow().contains(widget),
            None => true,
        });
        self.scope = EffectScope::new();
        self.bind(&self.bindings);
        self.bind_props(&self.props);
        self.bind_structures(&self.structures);
    }
}

fn reconcile_each(
    tree: &RefCell<WidgetTree>,
    parent: WidgetId,
    old_rows: &[RowInfo],
    new_widget: Widget,
    new_rows: &[RowInfo],
) -> Vec<Widget> {
    let Some(old_children) = tree.borrow_mut().take_children(parent) else {
        return new_widget.into_children();
    };
    let mut remaining = old_children;
    let mut old_widgets = Vec::with_capacity(old_rows.len());
    for row in old_rows {
        let take = row.count.min(remaining.len());
        old_widgets.push(remaining.drain(..take).collect::<Vec<_>>());
    }

    let mut remaining = new_widget.into_children();
    let mut new_widgets = Vec::with_capacity(new_rows.len());
    for row in new_rows {
        let take = row.count.min(remaining.len());
        new_widgets.push(remaining.drain(..take).collect::<Vec<_>>());
    }

    let mut used = vec![false; old_widgets.len()];
    let mut children = Vec::new();
    for (index, row) in new_rows.iter().enumerate() {
        let mut reused = None;
        if let Some(key) = &row.key {
            for (other, old) in old_rows.iter().enumerate() {
                if !used[other] && old.key.as_ref() == Some(key) && old.item == row.item {
                    reused = Some(other);
                    break;
                }
            }
        } else if let Some(old) = old_rows.get(index)
            && !used[index]
            && old.key.is_none()
            && old.item == row.item
        {
            reused = Some(index);
        }
        match reused {
            Some(other) => {
                used[other] = true;
                children.append(&mut old_widgets[other]);
            }
            None => children.append(&mut new_widgets[index]),
        }
    }
    children
}

fn handler_target(event: &WidgetEvent) -> Option<(Option<WidgetId>, HandlerEvent)> {
    match event {
        WidgetEvent::Click(id) => Some((Some(*id), HandlerEvent::Click)),
        WidgetEvent::Press(id) => Some((Some(*id), HandlerEvent::Press)),
        WidgetEvent::Release(id) => Some((Some(*id), HandlerEvent::Release)),
        WidgetEvent::PointerEnter(id) => Some((Some(*id), HandlerEvent::Enter)),
        WidgetEvent::PointerLeave(id) => Some((Some(*id), HandlerEvent::Leave)),
        WidgetEvent::Key(_) => Some((None, HandlerEvent::Key)),
        WidgetEvent::Drag(_) => None,
        WidgetEvent::Timer(_) => None,
    }
}

fn apply(binding: &Binding, component: &ComponentHandle, tree: &RefCell<WidgetTree>) {
    component.borrow().track();
    let count = binding.scopes.len();
    component.borrow_mut().push_binding_scopes(&binding.scopes);
    match &binding.target {
        BindingTarget::Text(expression) => {
            if let Some(content) = text(component, expression) {
                tree.borrow_mut().set_text(binding.widget, content);
            }
        }
        BindingTarget::Checked(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_checked(binding.widget, value);
            }
        }
        BindingTarget::Selected(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_selected(binding.widget, value);
            }
        }
        BindingTarget::Disabled(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_disabled(binding.widget, value);
            }
        }
        BindingTarget::Visible(expression) => {
            if let Some(value) = boolean(component, expression) {
                tree.borrow_mut().set_visible(binding.widget, value);
            }
        }
        BindingTarget::Offset { x, y } => {
            if let (Some(x), Some(y)) =
                (offset(component, x.as_ref()), offset(component, y.as_ref()))
            {
                tree.borrow_mut().set_offset(binding.widget, x, y);
            }
        }
    }
    component.borrow_mut().pop_binding_scopes(count);
}

fn apply_prop(prop: &PropBinding) {
    prop.component.borrow().track();
    let count = prop.scopes.len();
    prop.component
        .borrow_mut()
        .push_binding_scopes(&prop.scopes);
    if let Some(value) = evaluate(&prop.component, &prop.expression)
        && accepts_prop(&value, prop.ty)
        && let Err(error) = prop.target.borrow_mut().assign_property(&prop.name, value)
    {
        eprintln!("{error}");
    }
    prop.component.borrow_mut().pop_binding_scopes(count);
}

fn text(component: &ComponentHandle, expression: &Expr) -> Option<String> {
    match evaluate(component, expression)? {
        Value::String(content) => Some(content),
        _ => None,
    }
}

fn boolean(component: &ComponentHandle, expression: &Expr) -> Option<bool> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}

fn offset(component: &ComponentHandle, expression: Option<&Expr>) -> Option<i32> {
    let Some(expression) = expression else {
        return Some(0);
    };
    match evaluate(component, expression)? {
        Value::Number(value)
            if value.is_finite()
                && value.fract() == 0.0
                && value >= f64::from(i32::MIN)
                && value <= f64::from(i32::MAX) =>
        {
            Some(value as i32)
        }
        _ => None,
    }
}

/// Оценивает выражение в контексте компонента.
pub fn evaluate(component: &ComponentHandle, expression: &Expr) -> Option<Value> {
    eval(&mut component.borrow_mut(), expression).ok()
}

/// Запускает окно по модулю, сгенерированному из `.brul`.
pub fn run_generated(
    config: WindowConfig,
    script: Option<script::Program>,
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (component, built) = generated_built(view, theme, seeds, script.as_ref())?;
    let ui = Ui::assemble(component, built, view, theme.map(str::to_owned), host);
    run_ui(config, theme.map(str::to_owned), ui)
}

fn generated_built(
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    script: Option<&script::Program>,
) -> Result<(ComponentHandle, Built), Box<dyn std::error::Error>> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    for (name, value) in seeds {
        component.borrow_mut().declare(name, value.clone(), true)?;
    }
    if let Some(program) = script {
        execute_program(&mut component.borrow_mut(), program)?;
    }
    let built = view(theme, &component).map_err(std::io::Error::other)?;
    Ok((component, built))
}

fn run_ui(
    config: WindowConfig,
    theme: Option<String>,
    ui: Ui,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = Rc::new(RefCell::new(ui));
    let initial = ui.borrow_mut().dispatch(&[]);
    let builder = Rc::clone(&ui);
    let handler = Rc::clone(&ui);
    window::run_tree_themed(
        config,
        initial,
        move |scheme| {
            let mut ui = builder.borrow_mut();
            if let Err(error) = ui.set_theme(scheme_theme(theme.as_deref(), scheme))
                && !error.message.contains("unknown theme")
            {
                eprintln!("{error}");
            }
            Rc::clone(&ui.tree)
        },
        move |events| {
            let commands = handler.borrow_mut().dispatch(events);
            brul_reactive::flush();
            handler.borrow_mut().settle();
            commands
        },
    )?;
    Ok(())
}

fn scheme_theme(fixed: Option<&str>, scheme: window::ColorScheme) -> &str {
    fixed.unwrap_or(match scheme {
        window::ColorScheme::Light => "light",
        window::ColorScheme::Dark => "dark",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::{Direction, Widget};
    use brul_syntax::ast::{ExprKind, Position};

    fn generated_view(theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
        if theme.is_some() {
            return Err("unknown theme".to_owned());
        }
        let widget = Widget::text("");
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![Binding::new(
                component,
                id,
                BindingTarget::Text(Expr::new(
                    ExprKind::Ident("name".to_owned()),
                    Position::default(),
                )),
            )],
            handlers: Vec::new(),
            structures: Vec::new(),
            props: Vec::new(),
        })
    }

    fn assembled(
        theme: Option<&str>,
        seeds: &[(&str, Value)],
        script: Option<&script::Program>,
    ) -> (WidgetId, Ui) {
        let (component, built) =
            generated_built(generated_view, theme, seeds, script).expect("launch succeeds");
        let id = built.widget.id();
        (
            id,
            Ui::assemble(component, built, generated_view, None, None),
        )
    }

    #[test]
    fn generated_launch_declares_seeds() {
        let seeds = [("name", Value::String("from seed".to_owned()))];
        let (id, ui) = assembled(None, &seeds, None);
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from seed".to_owned()));
    }

    #[test]
    fn generated_launch_runs_the_script() {
        let program = script::Program {
            functions: Vec::new(),
            statements: vec![script::Statement::Let {
                constant: false,
                name: "name".to_owned(),
                value: Some(Expr::new(
                    ExprKind::String("from script".to_owned()),
                    Position::default(),
                )),
            }],
        };
        let (id, ui) = assembled(None, &[], Some(&program));
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from script".to_owned()));
    }

    #[test]
    fn explicit_theme_ignores_the_color_scheme() {
        assert_eq!(
            scheme_theme(Some("dark"), window::ColorScheme::Light),
            "dark"
        );
        assert_eq!(
            scheme_theme(Some("light"), window::ColorScheme::Dark),
            "light"
        );
    }

    #[test]
    fn system_theme_follows_the_color_scheme() {
        assert_eq!(scheme_theme(None, window::ColorScheme::Light), "light");
        assert_eq!(scheme_theme(None, window::ColorScheme::Dark), "dark");
    }

    fn ident(name: &str) -> Expr {
        Expr::new(ExprKind::Ident(name.to_owned()), Position::default())
    }

    fn shown(component: &ComponentHandle) -> Option<Value> {
        component.borrow().track();
        Some(Value::Bool(
            boolean(component, &ident("shown")).unwrap_or(false),
        ))
    }

    fn rebuild_shown(
        theme: Option<&str>,
        component: &ComponentHandle,
    ) -> Result<(Built, Vec<RowInfo>), String> {
        if theme.is_some() {
            return Err("unknown theme".to_owned());
        }
        let mut widget = Widget::view(Direction::Column, Vec::new());
        if boolean(component, &ident("shown")).unwrap_or(false) {
            widget.set_children(vec![Widget::text("shown")]);
        }
        Ok((
            Built {
                widget,
                bindings: Vec::new(),
                handlers: Vec::new(),
                structures: Vec::new(),
                props: Vec::new(),
            },
            Vec::new(),
        ))
    }

    fn structured_view(theme: Option<&str>, _component: &ComponentHandle) -> Result<Built, String> {
        if theme.is_some() {
            return Err("unknown theme".to_owned());
        }
        let widget = Widget::view(Direction::Column, Vec::new());
        let parent = widget.id();
        Ok(Built {
            widget,
            bindings: Vec::new(),
            handlers: Vec::new(),
            structures: vec![Structure {
                parent,
                each: false,
                value: shown,
                rebuild: rebuild_shown,
            }],
            props: Vec::new(),
        })
    }

    fn child_count(ui: &Ui, parent: WidgetId) -> usize {
        let children = ui
            .tree
            .borrow_mut()
            .take_children(parent)
            .unwrap_or_default();
        let count = children.len();
        ui.tree.borrow_mut().replace_all_children(parent, children);
        count
    }

    #[test]
    fn structure_rebuilds_when_its_value_changes() {
        let program = script::Program {
            functions: Vec::new(),
            statements: vec![script::Statement::Let {
                constant: false,
                name: "shown".to_owned(),
                value: Some(Expr::new(ExprKind::Bool(false), Position::default())),
            }],
        };
        let (component, built) =
            generated_built(structured_view, None, &[], Some(&program)).expect("launch succeeds");
        let parent = built.widget.id();
        let mut ui = Ui::assemble(component, built, structured_view, None, None);
        brul_reactive::flush();
        assert_eq!(child_count(&ui, parent), 0);

        ui.component
            .borrow_mut()
            .assign("shown", Value::Bool(true))
            .unwrap();
        brul_reactive::flush();
        ui.settle();
        assert_eq!(child_count(&ui, parent), 1);
    }

    #[test]
    fn each_reuses_rows_with_the_same_key() {
        let first = Widget::text("a");
        let second = Widget::text("b");
        let (first_id, second_id) = (first.id(), second.id());
        let root = Widget::view(Direction::Column, vec![first, second]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let row = |key: f64, item: &str, start: usize| RowInfo {
            key: Some(Value::Number(key)),
            item: Value::String(item.to_owned()),
            start,
            count: 1,
        };
        let old_rows = [row(1.0, "a", 0), row(2.0, "b", 1)];
        let new_rows = [row(2.0, "b", 0), row(1.0, "a", 1)];
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("b"), Widget::text("a")],
        );

        let children = reconcile_each(&tree, parent, &old_rows, fresh, &new_rows);
        let ids: Vec<WidgetId> = children.iter().map(Widget::id).collect();
        assert_eq!(ids, vec![second_id, first_id]);
    }

    fn declared(name: &str, value: Value) -> ComponentHandle {
        let mut component = Component::new();
        component.declare(name, value, false).expect("declared");
        Rc::new(RefCell::new(component))
    }

    #[test]
    fn handler_target_names_the_widget_of_a_pointer_event() {
        let id = Widget::text("x").id();
        let cases = [
            (WidgetEvent::Click(id), HandlerEvent::Click),
            (WidgetEvent::Press(id), HandlerEvent::Press),
            (WidgetEvent::Release(id), HandlerEvent::Release),
            (WidgetEvent::PointerEnter(id), HandlerEvent::Enter),
            (WidgetEvent::PointerLeave(id), HandlerEvent::Leave),
        ];
        for (event, expected) in cases {
            assert_eq!(handler_target(&event), Some((Some(id), expected)));
        }
    }

    #[test]
    fn handler_target_leaves_out_the_widget_for_keys_and_ignores_drag_and_timer() {
        assert_eq!(
            handler_target(&WidgetEvent::Key("Escape".to_owned())),
            Some((None, HandlerEvent::Key))
        );
        let id = Widget::text("x").id();
        assert_eq!(handler_target(&WidgetEvent::Drag(id)), None);
        assert_eq!(handler_target(&WidgetEvent::Timer("tick".to_owned())), None);
    }

    fn prop(
        component: &ComponentHandle,
        target: &ComponentHandle,
        name: &str,
        ty: PropType,
        expression: Expr,
    ) -> PropBinding {
        PropBinding {
            widget: Widget::text("").id(),
            target: Rc::clone(target),
            name: name.to_owned(),
            ty,
            expression,
            scopes: Vec::new(),
            component: Rc::clone(component),
        }
    }

    fn child_with_property(name: &str, value: Value) -> ComponentHandle {
        let mut component = Component::new();
        component
            .declare_property(name, value)
            .expect("declared property");
        Rc::new(RefCell::new(component))
    }

    #[test]
    fn a_prop_binding_writes_the_property_of_the_child() {
        let source = declared("colour", Value::String("red".to_owned()));
        let target = child_with_property("accent", Value::String("blue".to_owned()));
        apply_prop(&prop(
            &source,
            &target,
            "accent",
            PropType::String,
            ident("colour"),
        ));
        assert_eq!(
            target.borrow().read("accent").expect("read back"),
            Value::String("red".to_owned())
        );
    }

    #[test]
    fn a_prop_binding_skips_a_value_of_the_wrong_type() {
        let source = declared("colour", Value::String("red".to_owned()));
        let target = child_with_property("accent", Value::String("blue".to_owned()));
        apply_prop(&prop(
            &source,
            &target,
            "accent",
            PropType::Bool,
            ident("colour"),
        ));
        assert_eq!(
            target.borrow().read("accent").expect("read back"),
            Value::String("blue".to_owned())
        );
    }

    #[test]
    fn a_prop_binding_survives_an_unknown_property() {
        let source = declared("colour", Value::String("red".to_owned()));
        let target = child_with_property("accent", Value::String("blue".to_owned()));
        apply_prop(&prop(
            &source,
            &target,
            "missing",
            PropType::String,
            ident("colour"),
        ));
        assert!(target.borrow().read("missing").is_err());
    }

    #[test]
    fn an_absent_offset_is_zero() {
        let component = Component::new();
        assert_eq!(offset(&Rc::new(RefCell::new(component)), None), Some(0));
    }

    #[test]
    fn an_offset_needs_a_whole_number_in_range() {
        let mut component = Component::new();
        for (name, value) in [
            ("whole", Value::Number(4.0)),
            ("fraction", Value::Number(2.5)),
            ("nan", Value::Number(f64::NAN)),
            ("huge", Value::Number(3_000_000_000.0)),
            ("text", Value::String("4".to_owned())),
        ] {
            component.declare(name, value, false).expect("declared");
        }
        let component = Rc::new(RefCell::new(component));
        assert_eq!(offset(&component, Some(&ident("whole"))), Some(4));
        assert_eq!(offset(&component, Some(&ident("fraction"))), None);
        assert_eq!(offset(&component, Some(&ident("nan"))), None);
        assert_eq!(offset(&component, Some(&ident("huge"))), None);
        assert_eq!(offset(&component, Some(&ident("text"))), None);
        assert_eq!(offset(&component, Some(&ident("missing"))), None);
    }

    fn command_host(
        app: Option<Box<dyn Host>>,
    ) -> (CommandHost, Rc<RefCell<Vec<window::WindowCommand>>>) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        (
            CommandHost {
                commands: Rc::clone(&commands),
                app,
            },
            commands,
        )
    }

    struct Recorder {
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl Host for Recorder {
        fn call(&mut self, name: &str, _arguments: &[Value]) -> Result<Value, Error> {
            self.calls.borrow_mut().push(name.to_owned());
            Ok(Value::Null)
        }

        fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
            self.calls
                .borrow_mut()
                .push(format!("{function}@{interval_ms}"));
            Ok(())
        }
    }

    #[test]
    fn set_theme_takes_exactly_one_string() {
        let (mut host, commands) = command_host(None);
        assert_eq!(
            host.call("set_theme", &[]),
            Err(Error::Arity {
                name: "set_theme".to_owned(),
                expected: "one",
            })
        );
        assert_eq!(
            host.call("set_theme", &[Value::Number(1.0)]),
            Err(Error::Argument {
                name: "set_theme",
                expected: "String",
                found: "Number",
            })
        );
        assert_eq!(
            host.call("set_theme", &[Value::String("dark".to_owned())]),
            Ok(Value::Null)
        );
        assert_eq!(
            *commands.borrow(),
            vec![window::WindowCommand::SetTheme("dark".to_owned())]
        );
    }

    #[test]
    fn the_builtin_commands_take_no_arguments() {
        let (mut host, commands) = command_host(None);
        assert_eq!(host.call("exit", &[]), Ok(Value::Null));
        assert_eq!(host.call("dragWindow", &[]), Ok(Value::Null));
        assert_eq!(
            host.call("exit", &[Value::Number(1.0)]),
            Err(Error::Arity {
                name: "exit".to_owned(),
                expected: "no",
            })
        );
        assert_eq!(
            *commands.borrow(),
            vec![window::WindowCommand::Exit, window::WindowCommand::Drag]
        );
    }

    #[test]
    fn an_unknown_command_without_a_host_is_an_error() {
        let (mut host, _commands) = command_host(None);
        assert_eq!(
            host.call("theme", &[]),
            Err(Error::UnknownFunction("theme".to_owned()))
        );
        assert_eq!(host.every(16.0, "tick"), Err(Error::Unsupported("every")));
    }

    #[test]
    fn commands_and_timers_reach_the_application_host() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let recorder = Recorder {
            calls: Rc::clone(&calls),
        };
        let (mut host, _commands) = command_host(Some(Box::new(recorder)));
        assert_eq!(host.call("theme", &[]), Ok(Value::Null));
        assert_eq!(host.every(16.0, "tick"), Ok(()));
        assert_eq!(
            *calls.borrow(),
            vec!["theme".to_owned(), "tick@16".to_owned()]
        );
    }

    fn themed_view(theme: Option<&str>, _component: &ComponentHandle) -> Result<Built, String> {
        Ok(Built {
            widget: Widget::text(theme.unwrap_or("none")),
            bindings: Vec::new(),
            handlers: Vec::new(),
            structures: Vec::new(),
            props: Vec::new(),
        })
    }

    #[test]
    fn a_theme_change_rebuilds_the_view() {
        let (component, built) = generated_built(themed_view, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(component, built, themed_view, None, None);
        ui.set_theme("dark").expect("theme changes");
        assert_eq!(ui.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn a_failed_theme_change_keeps_the_previous_theme() {
        let (component, built) = generated_built(generated_view, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(
            component,
            built,
            generated_view,
            Some("light".to_owned()),
            None,
        );
        ui.set_theme("light").expect("same theme is a no-op");
        assert!(ui.set_theme("dark").is_err());
        assert_eq!(ui.theme.as_deref(), Some("light"));
    }
}

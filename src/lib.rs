//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use brul_codegen::runtime::{self, accepts_prop};
use brul_gui::{Widget, WidgetEvent, WidgetId, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
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
                Effect::new(move || runtime::apply(&binding, &tree));
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
            match event {
                WidgetEvent::Timer(function) => {
                    call(&Rc::clone(&self.component), function, Vec::new());
                }
                _ => self.dispatch_event(event),
            }
        }
        self.take_commands()
    }

    fn dispatch_event(&mut self, event: &WidgetEvent) {
        let Some((widget, target)) = handler_target(event) else {
            return;
        };
        let Some(handler) = self
            .handlers
            .iter()
            .find(|handler| handler.widget == widget && handler.event == target)
        else {
            return;
        };
        let component = Rc::clone(&handler.component);
        call(&component, &handler.function, arguments(event));
    }

    fn take_commands(&mut self) -> Vec<window::WindowCommand> {
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
    let mut old_widgets = group(old_children, old_rows);
    let mut new_widgets = group(new_widget.into_children(), new_rows);

    let mut used = vec![false; old_widgets.len()];
    let mut children = Vec::new();
    for (index, row) in new_rows.iter().enumerate() {
        match reuse_index(old_rows, row, index, &used) {
            Some(other) => {
                used[other] = true;
                children.append(&mut old_widgets[other]);
            }
            None => children.append(&mut new_widgets[index]),
        }
    }
    children
}

fn group(children: Vec<Widget>, rows: &[RowInfo]) -> Vec<Vec<Widget>> {
    let mut remaining = children;
    rows.iter()
        .map(|row| {
            let take = row.count.min(remaining.len());
            remaining.drain(..take).collect()
        })
        .collect()
}

fn reuse_index(old_rows: &[RowInfo], row: &RowInfo, index: usize, used: &[bool]) -> Option<usize> {
    let Some(key) = &row.key else {
        let same_place = index < old_rows.len()
            && !used[index]
            && old_rows[index].key.is_none()
            && old_rows[index].item == row.item;
        return same_place.then_some(index);
    };
    old_rows.iter().enumerate().position(|(other, old)| {
        !used[other] && old.key.as_ref() == Some(key) && old.item == row.item
    })
}

fn call(component: &ComponentHandle, function: &str, arguments: Vec<Value>) {
    if let Err(error) = component.borrow_mut().call_function(function, arguments) {
        eprintln!("{error}");
    }
}

fn arguments(event: &WidgetEvent) -> Vec<Value> {
    match event {
        WidgetEvent::Key(code) => vec![Value::Object(vec![(
            "code".to_owned(),
            Value::String(code.clone()),
        )])],
        _ => Vec::new(),
    }
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

/// Оценивает выражение в контексте компонента.
pub fn evaluate(component: &ComponentHandle, expression: &Expr) -> Option<Value> {
    runtime::evaluate(&mut component.borrow_mut(), expression).ok()
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
    use brul_gui::{Direction, Size, TextEngine, Widget};
    use brul_syntax::ast::{AssignOp, ExprKind, Position};

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
        let flag = evaluate(component, &ident("shown")).and_then(|value| value.as_flag());
        Some(Value::Bool(flag.unwrap_or(false)))
    }

    fn rebuild_shown(
        theme: Option<&str>,
        component: &ComponentHandle,
    ) -> Result<(Built, Vec<RowInfo>), String> {
        if theme.is_some() {
            return Err("unknown theme".to_owned());
        }
        let mut widget = Widget::view(Direction::Column, Vec::new());
        let flag = evaluate(component, &ident("shown")).and_then(|value| value.as_flag());
        if flag.unwrap_or(false) {
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

    fn binding(component: &ComponentHandle, widget: WidgetId, target: BindingTarget) -> Binding {
        Binding::new(component, widget, target)
    }

    fn state_after(targets: [BindingTarget; 6]) -> (Vec<WidgetId>, RefCell<WidgetTree>) {
        let mut component = Component::new();
        component
            .declare("title", Value::String("hi".to_owned()), false)
            .unwrap();
        component.declare("on", Value::Bool(true), false).unwrap();
        component.declare("off", Value::Bool(false), false).unwrap();
        component.declare("x", Value::Number(4.0), false).unwrap();
        component.declare("y", Value::Number(6.0), false).unwrap();
        let component = Rc::new(RefCell::new(component));
        let widgets: Vec<Widget> = (0..targets.len())
            .map(|index| Widget::text(index.to_string()))
            .collect();
        let ids: Vec<WidgetId> = widgets.iter().map(Widget::id).collect();
        let tree = RefCell::new(WidgetTree::new(Widget::view(Direction::Column, widgets)));
        for (index, target) in targets.into_iter().enumerate() {
            runtime::apply(&binding(&component, ids[index], target), &tree);
        }
        (ids, tree)
    }

    #[test]
    fn every_binding_target_writes_its_own_state() {
        let targets = [
            BindingTarget::Text(ident("title")),
            BindingTarget::Checked(ident("on")),
            BindingTarget::Selected(ident("on")),
            BindingTarget::Disabled(ident("on")),
            BindingTarget::Visible(ident("off")),
            BindingTarget::Offset {
                x: Some(ident("x")),
                y: Some(ident("y")),
            },
        ];
        let (ids, tree) = state_after(targets);
        assert!(!tree.borrow_mut().set_text(ids[0], "hi".to_owned()));
        let state = tree.borrow().state(ids[1]).expect("state");
        assert!(state.checked);
        assert!(tree.borrow().state(ids[2]).expect("state").selected);
        assert!(tree.borrow().state(ids[3]).expect("state").disabled);
        assert!(!tree.borrow().state(ids[4]).expect("state").visible);
        let mut engine = TextEngine::new();
        tree.borrow_mut()
            .layout(
                &mut engine,
                Size {
                    width: 200,
                    height: 200,
                },
                1.0,
            )
            .expect("layout");
        let mut tree = tree.borrow_mut();
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(ids[5]).cloned().expect("node");
        assert_eq!((node.x, node.y), (4, 6));
    }

    fn row(key: Option<Value>, item: &str, count: usize) -> RowInfo {
        RowInfo {
            key,
            item: Value::String(item.to_owned()),
            start: 0,
            count,
        }
    }

    #[test]
    fn each_without_old_children_builds_the_new_ones() {
        let tree = RefCell::new(WidgetTree::new(Widget::view(Direction::Column, Vec::new())));
        let absent = Widget::text("absent").id();
        let fresh = Widget::view(Direction::Column, vec![Widget::text("a")]);
        let new_id = fresh.id();
        let children = reconcile_each(
            &tree,
            absent,
            &[row(None, "a", 1)],
            fresh,
            &[row(None, "a", 1)],
        );
        assert_eq!(children.len(), 1);
        assert_ne!(children[0].id(), new_id);
    }

    #[test]
    fn an_unkeyed_row_reuses_the_widget_in_the_same_place() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &[row(None, "a", 1)],
        );
        assert_eq!(children[0].id(), first_id);
    }

    #[test]
    fn an_unkeyed_row_with_a_different_item_is_rebuilt() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &[row(None, "b", 1)],
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_is_rebuilt_when_its_item_changes() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let key = Some(Value::Number(1.0));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(key.clone(), "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &[row(key, "b", 1)],
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_takes_its_widget_only_once() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let key = Some(Value::Number(1.0));
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("a"), Widget::text("a")],
        );
        let children = reconcile_each(
            &tree,
            parent,
            &[row(key.clone(), "a", 1)],
            fresh,
            &[row(key.clone(), "a", 1), row(key, "a", 1)],
        );
        assert_eq!(children[0].id(), first_id);
        assert_ne!(children[1].id(), first_id);
    }

    #[test]
    fn a_keyed_row_does_not_take_an_unkeyed_widget() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &[row(Some(Value::Number(1.0)), "a", 1)],
        );
        assert_ne!(children[0].id(), first_id);
    }

    fn component_with(functions: Vec<script::Function>, declared: &[&str]) -> ComponentHandle {
        let mut component = Component::new();
        for name in declared {
            component
                .declare(name, Value::Null, false)
                .expect("declared");
        }
        for function in functions {
            component.define_function(function).expect("function");
        }
        Rc::new(RefCell::new(component))
    }

    fn function(name: &str, params: Vec<&str>, body: Vec<script::Statement>) -> script::Function {
        script::Function {
            name: name.to_owned(),
            params: params.into_iter().map(str::to_owned).collect(),
            body,
        }
    }

    fn let_true(name: &str) -> script::Statement {
        let value = Expr::new(ExprKind::Bool(true), Position::default());
        let target = Expr::new(ExprKind::Ident(name.to_owned()), Position::default());
        script::Statement::Expr(Expr::new(
            ExprKind::Assign {
                target: Box::new(target),
                op: AssignOp::Set,
                value: Box::new(value),
            },
            Position::default(),
        ))
    }

    fn dispatching(
        component: &ComponentHandle,
        widget: Widget,
        handlers: impl FnOnce(WidgetId) -> Vec<Handler>,
    ) -> (WidgetId, Ui) {
        let id = widget.id();
        let built = Built {
            widget,
            bindings: Vec::new(),
            handlers: handlers(id),
            structures: Vec::new(),
            props: Vec::new(),
        };
        (
            id,
            Ui::assemble(Rc::clone(component), built, generated_view, None, None),
        )
    }

    #[test]
    fn dispatch_calls_the_handler_of_the_widget() {
        let component = component_with(
            vec![function("clicked", Vec::new(), vec![let_true("ran")])],
            &["ran"],
        );
        let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
            vec![Handler::new(
                &component,
                Some(id),
                HandlerEvent::Click,
                "clicked",
            )]
        });
        ui.dispatch(&[WidgetEvent::Click(id)]);
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    }

    #[test]
    fn dispatch_passes_the_key_code() {
        let target = Expr::new(ExprKind::Ident("seen".to_owned()), Position::default());
        let value = Expr::new(ExprKind::Ident("code".to_owned()), Position::default());
        let body = vec![script::Statement::Expr(Expr::new(
            ExprKind::Assign {
                target: Box::new(target),
                op: AssignOp::Set,
                value: Box::new(value),
            },
            Position::default(),
        ))];
        let component = component_with(vec![function("pressed", vec!["code"], body)], &["seen"]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
            vec![Handler::new(&component, None, HandlerEvent::Key, "pressed")]
        });
        ui.dispatch(&[WidgetEvent::Key("Enter".to_owned())]);
        let seen = ui.component.borrow().read("seen").expect("seen");
        assert_eq!(
            seen,
            Value::Object(vec![("code".to_owned(), Value::String("Enter".to_owned()))])
        );
    }

    #[test]
    fn dispatch_runs_a_timer() {
        let component = component_with(
            vec![function("tick", Vec::new(), vec![let_true("ran")])],
            &["ran"],
        );
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
        ui.dispatch(&[WidgetEvent::Timer("tick".to_owned())]);
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    }

    #[test]
    fn dispatch_ignores_events_without_a_handler() {
        let component = component_with(Vec::new(), &[]);
        let (id, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
        let other = Widget::text("other").id();
        assert!(
            ui.dispatch(&[WidgetEvent::Drag(id), WidgetEvent::Click(other)])
                .is_empty()
        );
        assert!(ui.component.borrow().read("ran").is_err());
    }

    #[test]
    fn dispatch_returns_commands_it_cannot_handle() {
        let component = component_with(Vec::new(), &[]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
        ui.commands.borrow_mut().push(window::WindowCommand::Exit);
        assert_eq!(ui.dispatch(&[]), vec![window::WindowCommand::Exit]);
    }

    #[test]
    fn dispatch_applies_a_theme_command() {
        let (component, built) = generated_built(themed_view, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(component, built, themed_view, None, None);
        ui.commands
            .borrow_mut()
            .push(window::WindowCommand::SetTheme("dark".to_owned()));
        assert!(ui.dispatch(&[]).is_empty());
        assert_eq!(ui.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn dispatch_logs_a_theme_the_view_rejects() {
        let (component, built) = generated_built(generated_view, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(component, built, generated_view, None, None);
        ui.commands
            .borrow_mut()
            .push(window::WindowCommand::SetTheme("dark".to_owned()));
        assert!(ui.dispatch(&[]).is_empty());
        assert!(ui.theme.is_none());
    }
}

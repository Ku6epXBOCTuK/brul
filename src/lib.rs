//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use brul_compiler::{
    Binding, BindingTarget, Built, ComponentHandle, Handler, HandlerEvent, Loaded, RowInfo,
    Structure, StructureKind, build_bound_tree, rebuild_structure,
};
use brul_gui::{Widget, WidgetEvent, WidgetId, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Error, Host, Value, eval};
use brul_syntax::ast::Expr;
use brul_syntax::markup::Element;
use brul_syntax::style::Stylesheet;

pub struct Ui {
    pub component: ComponentHandle,
    pub tree: Rc<RefCell<WidgetTree>>,
    handlers: Vec<Handler>,
    window_handlers: Vec<Handler>,
    commands: Rc<RefCell<Vec<window::WindowCommand>>>,
    bindings: Vec<Binding>,
    structures: Vec<Structure>,
    root: Element,
    stylesheet: Stylesheet,
    theme: Option<String>,
    scope: EffectScope,
}

struct CommandHost {
    commands: Rc<RefCell<Vec<window::WindowCommand>>>,
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
            _ => return Err(Error::UnknownFunction(name.to_owned())),
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
}

impl Ui {
    pub fn new(loaded: Loaded) -> Self {
        let Loaded {
            component,
            tree: widget,
            bindings,
            handlers,
            structures,
            root,
            stylesheet,
            theme,
            ..
        } = loaded;
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
            structures,
            root,
            stylesheet,
            theme,
            scope: EffectScope::new(),
        };
        ui.bind(&ui.bindings);
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

    fn rebuild(&mut self) -> Result<(), brul_syntax::error::Error> {
        let built = build_bound_tree(
            &self.root,
            &self.stylesheet,
            self.theme.as_deref(),
            &self.component,
        )?;
        self.tree.borrow_mut().replace_root(built.widget);
        let mut handlers = built.handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.handlers = handlers;
        self.bindings = built.bindings;
        self.structures = built.structures;
        self.scope = EffectScope::new();
        self.bind(&self.bindings);
        Ok(())
    }

    fn set_theme(&mut self, theme: &str) -> Result<(), brul_syntax::error::Error> {
        let previous = self.theme.replace(theme.to_owned());
        if let Err(error) = self.rebuild() {
            self.theme = previous;
            return Err(error);
        }
        Ok(())
    }

    fn poll(&mut self) {
        let structures = std::mem::take(&mut self.structures);
        let mut kept = Vec::with_capacity(structures.len());
        let mut bindings = Vec::new();
        let mut handlers = Vec::new();
        let mut changed = false;
        for mut structure in structures {
            if !self.tree.borrow().contains(structure.parent) {
                continue;
            }
            let differs = match &structure.kind {
                StructureKind::If { block, last } => {
                    match evaluate(&structure.component, &block.condition) {
                        Some(Value::Bool(value)) => value != *last,
                        _ => false,
                    }
                }
                StructureKind::Each { block, last, .. } => {
                    match evaluate(&structure.component, &block.collection) {
                        Some(Value::Array(items)) => items != *last,
                        _ => false,
                    }
                }
            };
            if !differs {
                kept.push(structure);
                continue;
            }
            let built = rebuild_structure(
                &structure,
                &self.stylesheet,
                self.theme.as_deref(),
                &structure.component,
            );
            match built {
                Ok((built, last, new_rows)) => {
                    let Built {
                        widget,
                        bindings: added_bindings,
                        handlers: added_handlers,
                        structures: added_structures,
                    } = built;
                    let children = match &structure.kind {
                        StructureKind::Each { rows, .. } => {
                            reconcile_each(&self.tree, structure.parent, rows, widget, &new_rows)
                        }
                        _ => widget.into_children(),
                    };
                    self.tree
                        .borrow_mut()
                        .replace_all_children(structure.parent, children);
                    match (&mut structure.kind, last) {
                        (StructureKind::If { last, .. }, Value::Bool(value)) => *last = value,
                        (StructureKind::Each { last, rows, .. }, Value::Array(items)) => {
                            *last = items;
                            *rows = new_rows;
                        }
                        _ => {}
                    }
                    bindings.extend(added_bindings);
                    handlers.extend(added_handlers);
                    kept.extend(added_structures);
                    kept.push(structure);
                    changed = true;
                }
                Err(error) => {
                    eprintln!("{error}");
                    kept.push(structure);
                }
            }
        }
        self.structures = kept;
        if changed {
            let tree = Rc::clone(&self.tree);
            self.bindings.extend(bindings);
            self.bindings
                .retain(|binding| tree.borrow().contains(binding.widget));
            self.handlers.extend(handlers);
            self.handlers.retain(|handler| match handler.widget {
                Some(widget) => tree.borrow().contains(widget),
                None => true,
            });
            self.scope = EffectScope::new();
            self.bind(&self.bindings);
        }
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
        self.poll();
        output
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

fn apply(binding: &Binding, component: &RefCell<Component>, tree: &RefCell<WidgetTree>) {
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

fn text(component: &RefCell<Component>, expression: &Expr) -> Option<String> {
    match evaluate(component, expression)? {
        Value::String(content) => Some(content),
        _ => None,
    }
}

fn boolean(component: &RefCell<Component>, expression: &Expr) -> Option<bool> {
    match evaluate(component, expression)? {
        Value::Bool(value) => Some(value),
        _ => None,
    }
}

fn offset(component: &RefCell<Component>, expression: Option<&Expr>) -> Option<i32> {
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

fn evaluate(component: &RefCell<Component>, expression: &Expr) -> Option<Value> {
    eval(&mut component.borrow_mut(), expression).ok()
}

/// Загружает `.brul` и открывает его окно.
pub fn run(source: &str, theme: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let loaded = brul_compiler::load(source, theme)?;
    let config = loaded.config.clone();
    let mut ui = Ui::new(loaded);
    let initial = ui.dispatch(&[]);
    let tree = Rc::clone(&ui.tree);
    window::run_tree(config, tree, initial, move |events| ui.dispatch(events))?;
    Ok(())
}

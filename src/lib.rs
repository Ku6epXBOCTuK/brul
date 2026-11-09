//! Фасад BRUL: запуск приложения, корни окон, общий контекст.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use brul_compiler::{Binding, BindingTarget, Handler, HandlerEvent, Loaded, build_bound_tree};
use brul_gui::{Widget, WidgetEvent, WidgetId, WidgetTree, window};
use brul_reactive::{Effect, EffectScope};
use brul_script::{Component, Error, Host, Value, eval};
use brul_syntax::ast::Expr;
use brul_syntax::markup::Element;
use brul_syntax::style::Stylesheet;

pub struct Ui {
    pub component: Rc<RefCell<Component>>,
    pub tree: Rc<RefCell<WidgetTree>>,
    handlers: Vec<Handler>,
    window_handlers: Vec<Handler>,
    commands: Rc<RefCell<Vec<window::WindowCommand>>>,
    root: Element,
    stylesheet: Stylesheet,
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
    pub fn new(
        component: Component,
        widget: Widget,
        bindings: Vec<Binding>,
        handlers: Vec<Handler>,
        root: Element,
        stylesheet: Stylesheet,
    ) -> Self {
        let mut component = component;
        let commands = Rc::new(RefCell::new(Vec::new()));
        for (interval_ms, function) in component.take_every() {
            if interval_ms.is_finite() && interval_ms > 0.0 {
                commands.borrow_mut().push(window::WindowCommand::Every {
                    interval: Duration::from_secs_f64(interval_ms / 1000.0),
                    function,
                });
            } else {
                eprintln!("every interval must be a positive number");
            }
        }
        component.set_host(CommandHost {
            commands: Rc::clone(&commands),
        });
        let component = Rc::new(RefCell::new(component));
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
            root,
            stylesheet,
            scope: EffectScope::new(),
        };
        ui.bind(bindings);
        ui
    }

    fn bind(&self, bindings: Vec<Binding>) {
        for binding in bindings {
            let component = Rc::clone(&self.component);
            let tree = Rc::clone(&self.tree);
            self.scope.run(|| {
                Effect::new(move || apply(&binding, &component, &tree));
            });
        }
    }

    fn set_theme(&mut self, theme: &str) -> Result<(), brul_syntax::error::Error> {
        let built = {
            let mut component = self.component.borrow_mut();
            build_bound_tree(&self.root, &self.stylesheet, Some(theme), &mut component)?
        };
        self.tree.borrow_mut().replace_root(built.widget);
        let mut handlers = built.handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.handlers = handlers;
        self.scope = EffectScope::new();
        self.bind(built.bindings);
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
            let arguments = match event {
                WidgetEvent::Key(code) => vec![Value::Object(vec![(
                    "code".to_owned(),
                    Value::String(code.clone()),
                )])],
                _ => Vec::new(),
            };
            if let Err(error) = self
                .component
                .borrow_mut()
                .call_function(&function, arguments)
            {
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
    let Loaded {
        config,
        tree,
        bindings,
        handlers,
        component,
        root,
        stylesheet,
        ..
    } = brul_compiler::load(source, theme)?;
    let mut ui = Ui::new(component, tree, bindings, handlers, root, stylesheet);
    let initial = ui.dispatch(&[]);
    let tree = Rc::clone(&ui.tree);
    window::run_tree(config, tree, initial, move |events| ui.dispatch(events))?;
    Ok(())
}

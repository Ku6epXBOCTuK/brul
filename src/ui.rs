use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use brul_codegen::runtime::{self, accepts_prop};
use brul_gui::{WidgetEvent, WidgetId, WidgetTree};
use brul_reactive::{Effect, EffectScope};
use brul_window::{WindowCommand, WindowKey};

use crate::each::reconcile_each;
use crate::event::{arguments, call, handler_target};
use crate::{
    Binding, Build, Built, ComponentHandle, Error, Handler, HandlerEvent, Host, PropBinding,
    RowInfo, Structure, Theme, Value, ViewError,
};

const EVENT_DEPTH: usize = 64;

const BUILTINS: &[&str] = &[
    "theme",
    "setTheme",
    "setAlwaysOnTop",
    "openWindow",
    "exit",
    "closeWindow",
    "dragWindow",
    "showWindow",
    "hideWindow",
    "focusWindow",
];

pub struct Ui<T: Theme> {
    pub component: ComponentHandle,
    pub tree: Rc<RefCell<WidgetTree>>,
    handlers: Vec<Handler>,
    window_handlers: Vec<Handler>,
    handler_index: HashMap<(Option<WidgetId>, HandlerEvent), usize>,
    pub(crate) commands: Rc<RefCell<Vec<WindowCommand>>>,
    bindings: Vec<Owned>,
    props: Vec<Owned>,
    structures: Vec<OwnedStructure>,
    rows: Vec<(WidgetId, Vec<RowInfo>)>,
    requests: Rc<RefCell<Vec<WidgetId>>>,
    view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
    pub(crate) theme: Option<T>,
    textures: runtime::Textures,
    scope: EffectScope,
}

struct Owned {
    widget: WidgetId,
    _scope: EffectScope,
}

struct OwnedStructure {
    structure: Structure,
    _scope: EffectScope,
}

pub(crate) struct CommandHost {
    pub(crate) commands: Rc<RefCell<Vec<WindowCommand>>>,
    pub(crate) app: Option<Box<dyn Host>>,
    pub(crate) windows: Rc<HashMap<String, WindowKey>>,
    pub(crate) theme: Rc<RefCell<Option<String>>>,
}

impl Host for CommandHost {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        tracing::debug!(name, arguments = arguments.len(), "host call");
        if name == "theme" {
            if !arguments.is_empty() {
                return Err(Error::Arity {
                    name: name.to_owned(),
                    expected: "no",
                });
            }
            return Ok(match &*self.theme.borrow() {
                Some(theme) => Value::String(theme.clone().into()),
                None => Value::Null,
            });
        }
        if name == "setTheme" {
            if arguments.len() != 1 {
                return Err(Error::Arity {
                    name: name.to_owned(),
                    expected: "one",
                });
            }
            let Value::String(theme) = &arguments[0] else {
                return Err(Error::Argument {
                    name: "setTheme",
                    expected: "String",
                    found: arguments[0].type_name(),
                });
            };
            self.commands
                .borrow_mut()
                .push(WindowCommand::SetTheme(theme.to_string()));
            return Ok(Value::Null);
        }
        if name == "setAlwaysOnTop" {
            if arguments.len() != 1 {
                return Err(Error::Arity {
                    name: name.to_owned(),
                    expected: "one",
                });
            }
            let Value::Bool(on_top) = &arguments[0] else {
                return Err(Error::Argument {
                    name: "setAlwaysOnTop",
                    expected: "Bool",
                    found: arguments[0].type_name(),
                });
            };
            self.commands
                .borrow_mut()
                .push(WindowCommand::AlwaysOnTop(*on_top));
            return Ok(Value::Null);
        }
        if name == "openWindow" {
            return self.open_window(arguments);
        }
        let command = match name {
            "exit" | "closeWindow" => WindowCommand::Close,
            "dragWindow" => WindowCommand::Drag,
            "showWindow" => WindowCommand::Show,
            "hideWindow" => WindowCommand::Hide,
            "focusWindow" => WindowCommand::Focus,
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

    fn commands(&self) -> Vec<&'static str> {
        let mut names = BUILTINS.to_vec();
        if let Some(app) = &self.app {
            names.extend(app.commands());
        }
        names
    }
}

impl CommandHost {
    fn open_window(&mut self, arguments: &[Value]) -> Result<Value, Error> {
        if arguments.len() != 1 {
            return Err(Error::Arity {
                name: "openWindow".to_owned(),
                expected: "one",
            });
        }
        let Value::String(name) = &arguments[0] else {
            return Err(Error::Argument {
                name: "openWindow",
                expected: "String",
                found: arguments[0].type_name(),
            });
        };
        let Some(key) = self.windows.get(&**name) else {
            return Err(Error::UnknownWindow(name.to_string()));
        };
        self.commands.borrow_mut().push(WindowCommand::Open(*key));
        Ok(Value::Null)
    }

    fn delegate(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error> {
        match &mut self.app {
            Some(host) => host.call(name, arguments),
            None => Err(Error::UnknownFunction(name.to_owned())),
        }
    }
}

impl<T: Theme> Ui<T> {
    #[cfg(test)]
    pub(crate) fn assemble(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Option<T>,
        structures: Vec<Structure>,
        host: Option<Box<dyn Host>>,
    ) -> Self {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let ui = Self::assemble_with(
            component,
            built,
            view,
            theme,
            structures,
            Rc::clone(&commands),
            true,
        );
        ui.component.borrow_mut().set_host(CommandHost {
            commands,
            app: host,
            windows: Rc::new(HashMap::new()),
            theme: Rc::new(RefCell::new(theme.map(Theme::name).map(str::to_owned))),
        });
        ui
    }

    pub(crate) fn assemble_with(
        component: ComponentHandle,
        built: Built,
        view: fn(Option<T>, &mut Build, &ComponentHandle) -> Result<Built, ViewError>,
        theme: Option<T>,
        structures: Vec<Structure>,
        commands: Rc<RefCell<Vec<WindowCommand>>>,
        scroll: bool,
    ) -> Self {
        let Built {
            widget,
            bindings,
            handlers,
            props,
        } = built;
        for (interval_ms, function) in component.borrow_mut().take_every() {
            if interval_ms.is_finite() && interval_ms > 0.0 {
                commands.borrow_mut().push(WindowCommand::Every {
                    interval: Duration::from_secs_f64(interval_ms / 1000.0),
                    function,
                });
            } else {
                tracing::error!(interval_ms, "every interval must be a positive number");
            }
        }
        let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
        tree.borrow_mut().set_root_scroll(scroll);
        let window_handlers = handlers
            .iter()
            .filter(|handler| handler.widget.is_none())
            .cloned()
            .collect();
        let mut ui = Self {
            component,
            tree,
            handlers,
            window_handlers,
            handler_index: HashMap::new(),
            commands,
            bindings: Vec::new(),
            props: Vec::new(),
            structures: Vec::new(),
            rows: Vec::new(),
            requests: Rc::new(RefCell::new(Vec::new())),
            view,
            theme,
            textures: runtime::Textures::default(),
            scope: EffectScope::new(),
        };
        ui.bind(bindings);
        ui.bind_props(props);
        ui.bind_structures(structures);
        ui.index_handlers();
        ui
    }

    fn index_handlers(&mut self) {
        self.handler_index = self
            .handlers
            .iter()
            .enumerate()
            .rev()
            .map(|(at, handler)| ((handler.widget, handler.event), at))
            .collect();
    }

    fn bind(&mut self, bindings: Vec<Binding>) {
        for binding in bindings {
            let widget = binding.widget;
            let tree = Rc::clone(&self.tree);
            let textures = Rc::clone(&self.textures);
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || {
                    let pending = runtime::evaluate(&binding, &textures);
                    runtime::write(pending, binding.widget, &mut tree.borrow_mut());
                });
            });
            self.bindings.push(Owned {
                widget,
                _scope: scope,
            });
        }
    }

    fn bind_props(&mut self, props: Vec<PropBinding>) {
        for prop in props {
            let widget = prop.widget;
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || apply_prop(&prop));
            });
            self.props.push(Owned {
                widget,
                _scope: scope,
            });
        }
    }

    fn bind_structures(&mut self, structures: Vec<Structure>) {
        for structure in structures {
            let component = Rc::clone(&structure.component);
            let each_scope = structure.scope.clone();
            let structure_value = Rc::clone(&structure.value);
            let requests = Rc::clone(&self.requests);
            let initialized = Rc::new(Cell::new(false));
            let last: Rc<Cell<Option<Value>>> = Rc::new(Cell::new(None));
            let scope = self.scope.child();
            scope.run(|| {
                Effect::new(move || {
                    let previous = component.borrow().scope().clone();
                    component.borrow_mut().set_scope(each_scope.clone());
                    let evaluated = structure_value.call(&mut component.borrow_mut());
                    component.borrow_mut().set_scope(previous);
                    let value = match evaluated {
                        Ok(value) => Some(value),
                        Err(error) => {
                            tracing::error!(%error, "structure expression failed");
                            None
                        }
                    };
                    if !initialized.replace(true) {
                        last.set(value);
                        return;
                    }
                    let changed = last.take().as_ref() != value.as_ref();
                    last.set(value);
                    if !changed {
                        return;
                    }
                    let mut requests = requests.borrow_mut();
                    if !requests.contains(&structure.parent) {
                        requests.push(structure.parent);
                    }
                });
            });
            self.structures.push(OwnedStructure {
                structure,
                _scope: scope,
            });
        }
    }

    fn rebuild(&mut self) -> Result<(), ViewError> {
        let mut build = Build::new();
        let built = (self.view)(self.theme, &mut build, &self.component);
        let structures = build.take_structures();
        let built = built?;
        self.tree.borrow_mut().replace_root(built.widget);
        let mut handlers = built.handlers;
        handlers.extend(self.window_handlers.iter().cloned());
        self.handlers = handlers;
        self.index_handlers();
        self.bindings.clear();
        self.props.clear();
        self.structures.clear();
        self.rows.clear();
        self.requests.borrow_mut().clear();
        self.scope = EffectScope::new();
        self.bind(built.bindings);
        self.bind_props(built.props);
        self.bind_structures(structures);
        Ok(())
    }

    pub(crate) fn set_theme(&mut self, theme: T) -> Result<(), ViewError> {
        if self.theme == Some(theme) {
            return Ok(());
        }
        let previous = self.theme.replace(theme);
        if let Err(error) = self.rebuild() {
            self.theme = previous;
            return Err(error);
        }
        Ok(())
    }

    pub fn dispatch(&mut self, events: &[WidgetEvent]) -> Vec<WindowCommand> {
        for event in events {
            match event {
                WidgetEvent::Timer(function) => {
                    let call = self.component.borrow().event_call(function);
                    if let Some(call) = call {
                        let _ = call(&mut self.component.borrow_mut(), &[]);
                    }
                }
                WidgetEvent::Close => self.close(),
                _ => self.dispatch_event(event),
            }
            self.pump_events();
        }
        self.take_commands()
    }

    fn pump_events(&mut self) {
        for _ in 0..EVENT_DEPTH {
            let Some(event) = runtime::take_event() else {
                return;
            };
            call(&event.component, event.call, event.arguments);
            brul_reactive::flush();
        }
        tracing::error!(depth = EVENT_DEPTH, "event chain is too deep");
    }

    fn dispatch_event(&mut self, event: &WidgetEvent) {
        let Some((widget, target)) = handler_target(event) else {
            return;
        };
        let Some(&at) = self.handler_index.get(&(widget, target)) else {
            return;
        };
        let handler = &self.handlers[at];
        call(&handler.component, handler.call, arguments(event));
    }

    fn close(&mut self) {
        match self.handler_index.get(&(None, HandlerEvent::Close)) {
            Some(&at) => {
                let handler = &self.handlers[at];
                call(&handler.component, handler.call, Vec::new());
            }
            None => self.commands.borrow_mut().push(WindowCommand::Close),
        }
    }

    fn take_commands(&mut self) -> Vec<WindowCommand> {
        std::mem::take(&mut *self.commands.borrow_mut())
    }

    pub fn settle(&mut self) {
        self.pump_events();
        let requests = std::mem::take(&mut *self.requests.borrow_mut());
        if requests.is_empty() {
            return;
        }
        let structures = std::mem::take(&mut self.structures);
        let mut rows = std::mem::take(&mut self.rows);
        let mut bindings = Vec::new();
        let mut handlers = Vec::new();
        let mut props = Vec::new();
        let mut added_structures = Vec::new();
        let mut changed = false;
        let index: HashMap<WidgetId, usize> = structures
            .iter()
            .enumerate()
            .map(|(at, owned)| (owned.structure.parent, at))
            .collect();
        for parent in requests {
            let Some(&at) = index.get(&parent) else {
                continue;
            };
            if !self.tree.borrow().contains(parent) {
                continue;
            }
            let structure = structures[at].structure.clone();
            let component = Rc::clone(&structure.component);
            let old_rows = if structure.each {
                rows.iter()
                    .position(|(id, _)| *id == parent)
                    .map(|position| rows.remove(position).1)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let previous = component.borrow().scope().clone();
            component.borrow_mut().set_scope(structure.scope.clone());
            let mut build = Build::with_slots(structure.slots.clone());
            let rebuilt = (structure.rebuild)(
                self.theme.map(Theme::name),
                &mut build,
                &structure.component,
                &old_rows,
            );
            let added = build.take_structures();
            component.borrow_mut().set_scope(previous);
            let (built, rebuilt_rows) = match rebuilt {
                Ok(result) => result,
                Err(message) => {
                    tracing::error!(%message, "view rebuild failed");
                    if structure.each {
                        rows.push((parent, old_rows));
                    }
                    continue;
                }
            };
            let Built {
                widget,
                bindings: added_bindings,
                handlers: added_handlers,
                props: added_props,
            } = built;
            let mut tree = self.tree.borrow_mut();
            let children = if structure.each {
                rows.push((parent, rebuilt_rows.clone()));
                reconcile_each(&mut tree, parent, &old_rows, widget, &rebuilt_rows)
            } else {
                widget.into_children()
            };
            tree.replace_all_children(parent, children);
            bindings.extend(added_bindings);
            handlers.extend(added_handlers);
            props.extend(added_props);
            added_structures.extend(added);
            changed = true;
        }
        self.structures = structures;
        self.rows = rows;
        if !changed {
            return;
        }
        let parents: HashSet<WidgetId> = added_structures
            .iter()
            .map(|structure| structure.parent)
            .collect();
        {
            let tree = self.tree.borrow();
            self.structures.retain(|owned| {
                tree.contains(owned.structure.parent) && !parents.contains(&owned.structure.parent)
            });
            self.rows.retain(|(id, _)| tree.contains(*id));
            let widgets: HashSet<WidgetId> =
                bindings.iter().map(|binding| binding.widget).collect();
            self.bindings
                .retain(|owned| tree.contains(owned.widget) && !widgets.contains(&owned.widget));
            let widgets: HashSet<WidgetId> = props.iter().map(|prop| prop.widget).collect();
            self.props
                .retain(|owned| tree.contains(owned.widget) && !widgets.contains(&owned.widget));
            self.handlers.extend(handlers);
            self.handlers.retain(|handler| match handler.widget {
                Some(widget) => tree.contains(widget),
                None => true,
            });
        }
        self.index_handlers();
        self.bind(bindings);
        self.bind_props(props);
        self.bind_structures(added_structures);
    }
}

pub(crate) fn apply_prop(prop: &PropBinding) {
    let previous = prop.component.borrow().scope().clone();
    prop.component.borrow_mut().set_scope(prop.scope.clone());
    let value = match prop.expression.evaluate(&mut prop.component.borrow_mut()) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::error!(%error, "binding expression failed");
            None
        }
    };
    let id = prop.target.borrow().id(prop.name);
    if let Some(value) = value
        && accepts_prop(&value, &prop.ty)
        && let Some(id) = id
        && let Err(error) = prop.target.borrow_mut().assign_property(id, value)
    {
        tracing::error!(%error, "property assignment failed");
    }
    prop.component.borrow_mut().set_scope(previous);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scope, TestTheme};

    use std::cell::RefCell;
    use std::rc::Rc;

    use brul_gui::{Direction, Size, TextEngine, Texture, Widget};
    use brul_syntax::markup::PropType;
    use brul_window::WindowCommand;

    use crate::run::generated_built;
    use crate::{Binding, BindingTarget, Component, Dyn, HandlerEvent, Script};

    const NAMES: &[&str] = &[
        "shown", "label", "colour", "title", "on", "off", "x", "y", "ran", "seen", "code",
        "accent", "missing", "icon",
    ];

    fn unknown_theme() -> ViewError {
        ViewError::from(crate::gui::style::StyleError::from(
            crate::style::SnapshotError::UnknownTheme("sepia".to_owned()),
        ))
    }

    macro_rules! typed_read {
        ($macro:ident, $name:literal, $ty:ty) => {{
            fn read(component: &mut Component) -> Result<$ty, Error> {
                (&component.read($name)?).try_into()
            }
            read
        }};
    }

    macro_rules! text {
        ($name:literal) => {
            typed_read!(text, $name, String)
        };
    }

    macro_rules! flag {
        ($name:literal) => {
            typed_read!(flag, $name, bool)
        };
    }

    macro_rules! number {
        ($name:literal) => {
            typed_read!(number, $name, f64)
        };
    }

    fn shown(component: &mut Component) -> Result<Value, Error> {
        let flag = component.read("shown")?.as_flag();
        Ok(Value::Bool(flag.unwrap_or(false)))
    }

    fn rebuild_shown(
        theme: Option<&str>,
        _build: &mut Build,
        component: &ComponentHandle,
        _old_rows: &[RowInfo],
    ) -> Result<(Built, Vec<RowInfo>), ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let mut widget = Widget::view(Direction::Column, Vec::new());
        let flag = shown(&mut component.borrow_mut())
            .ok()
            .and_then(|value| value.as_flag());
        if flag.unwrap_or(false) {
            widget.set_children(vec![Widget::text("shown")]);
        }
        Ok((
            Built {
                widget,
                bindings: Vec::new(),
                handlers: Vec::new(),
                props: Vec::new(),
            },
            Vec::new(),
        ))
    }

    fn structured_view(
        theme: Option<TestTheme>,
        build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let widget = Widget::view(Direction::Column, Vec::new());
        runtime::register_structure(build, component, widget.id(), false, shown, rebuild_shown);
        Ok(Built {
            widget,
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn child_count<T: Theme>(ui: &Ui<T>, parent: WidgetId) -> usize {
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
        let (component, built, structures) = generated_built(
            structured_view,
            NAMES,
            None,
            &[],
            Some(script_with(shown_false)),
        )
        .expect("launch succeeds");
        let parent = built.widget.id();
        let mut ui = Ui::assemble(component, built, structured_view, None, structures, None);
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

    fn slot_child_view(
        _theme: Option<&str>,
        build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        let holder = Widget::view(Direction::Column, Vec::new());
        runtime::register_structure(build, component, holder.id(), false, shown, rebuild_slot);
        Ok(Built {
            widget: holder,
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn rebuild_slot(
        _theme: Option<&str>,
        build: &mut Build,
        _component: &ComponentHandle,
        _old_rows: &[RowInfo],
    ) -> Result<(Built, Vec<RowInfo>), ViewError> {
        let built = runtime::slot(None, build, runtime::DEFAULT_SLOT)?;
        Ok((
            Built {
                widget: Widget::view(Direction::Column, built.widget.into_children()),
                bindings: built.bindings,
                handlers: built.handlers,
                props: built.props,
            },
            Vec::new(),
        ))
    }

    fn slot_fill(
        _theme: Option<&str>,
        _build: &mut Build,
        _component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        Ok(Built {
            widget: Widget::view(Direction::Column, vec![Widget::text("filled")]),
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn slot_view(
        theme: Option<TestTheme>,
        build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        let slots = [runtime::SlotArgument {
            name: runtime::DEFAULT_SLOT,
            view: slot_fill,
        }];
        runtime::instance(
            build,
            component,
            runtime::Instance {
                theme: theme.map(Theme::name),
                view: slot_child_view,
                script: None,
                name: "Slot",
                names: NAMES,
                props: &[],
                defaults: &[],
                events: &[],
                slots: &slots,
            },
        )
    }

    #[test]
    fn a_slot_survives_a_structure_rebuild() {
        let (_component, _built, structures) =
            generated_built(slot_view, NAMES, None, &[], None).expect("launch");
        assert_eq!(structures.len(), 1);
        let mut build = Build::with_slots(structures[0].slots.clone());
        let rebuilt = (structures[0].rebuild)(None, &mut build, &structures[0].component, &[])
            .expect("rebuild");
        assert_eq!(rebuilt.0.widget.into_children().len(), 1);
    }

    fn bound_view(
        theme: Option<TestTheme>,
        build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let holder = Widget::view(Direction::Column, Vec::new());
        runtime::register_structure(build, component, holder.id(), false, shown, rebuild_bound);
        Ok(Built {
            widget: holder,
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn rebuild_bound(
        theme: Option<&str>,
        _build: &mut Build,
        component: &ComponentHandle,
        _old_rows: &[RowInfo],
    ) -> Result<(Built, Vec<RowInfo>), ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let mut children = Vec::new();
        let mut bindings = Vec::new();
        let visible = shown(&mut component.borrow_mut())
            .ok()
            .and_then(|value| value.as_flag())
            .unwrap_or(false);
        if visible {
            let text = Widget::text("bound");
            bindings.push(Binding::new(
                component,
                text.id(),
                BindingTarget::Text(text!("label")),
            ));
            children.push(text);
        }
        Ok((
            Built {
                widget: Widget::view(Direction::Column, children),
                bindings,
                handlers: Vec::new(),
                props: Vec::new(),
            },
            Vec::new(),
        ))
    }

    #[test]
    fn the_effect_of_a_removed_branch_dies_with_its_widget() {
        let (component, built, structures) = generated_built(
            bound_view,
            NAMES,
            None,
            &[],
            Some(script_with(shown_and_label)),
        )
        .expect("launch succeeds");
        let mut ui = Ui::assemble(component, built, bound_view, None, structures, None);
        brul_reactive::flush();
        let assembled = brul_reactive::live_nodes();

        ui.component
            .borrow_mut()
            .assign("shown", Value::Bool(true))
            .unwrap();
        brul_reactive::flush();
        ui.settle();
        assert_eq!(brul_reactive::live_nodes(), assembled + 1);

        ui.component
            .borrow_mut()
            .assign("shown", Value::Bool(false))
            .unwrap();
        brul_reactive::flush();
        ui.settle();
        assert_eq!(brul_reactive::live_nodes(), assembled);
    }

    fn declared(name: &'static str, value: Value) -> ComponentHandle {
        let mut component = Component::new();
        component.set_names(NAMES);
        component.declare(name, value, false).expect("declared");
        Rc::new(RefCell::new(component))
    }

    fn prop(
        component: &ComponentHandle,
        target: &ComponentHandle,
        name: &'static str,
        ty: PropType,
        expression: brul_codegen::runtime::PropExpression,
    ) -> PropBinding {
        PropBinding {
            widget: Widget::text("").id(),
            target: Rc::clone(target),
            name,
            ty,
            expression,
            scope: Scope::default(),
            component: Rc::clone(component),
        }
    }

    fn child_with_property(name: &str, value: Value) -> ComponentHandle {
        let mut component = Component::new();
        component.set_names(NAMES);
        let id = component.id(name).expect("prop name is in the table");
        component
            .declare_property(id, value)
            .expect("declared property");
        Rc::new(RefCell::new(component))
    }

    #[test]
    fn a_prop_binding_writes_the_property_of_the_child() {
        let source = declared("colour", Value::String("red".into()));
        let target = child_with_property("accent", Value::String("blue".into()));
        apply_prop(&prop(
            &source,
            &target,
            "accent",
            PropType::String,
            brul_codegen::runtime::PropExpression::Text(text!("colour")),
        ));
        assert_eq!(
            target.borrow().read("accent").expect("read back"),
            Value::String("red".into())
        );
    }

    #[test]
    fn a_prop_binding_skips_a_value_of_the_wrong_type() {
        let source = declared("colour", Value::String("red".into()));
        let target = child_with_property("accent", Value::String("blue".into()));
        apply_prop(&prop(
            &source,
            &target,
            "accent",
            PropType::Bool,
            brul_codegen::runtime::PropExpression::Text(text!("colour")),
        ));
        assert_eq!(
            target.borrow().read("accent").expect("read back"),
            Value::String("blue".into())
        );
    }

    #[test]
    fn a_prop_binding_survives_an_unknown_property() {
        let source = declared("colour", Value::String("red".into()));
        let target = child_with_property("accent", Value::String("blue".into()));
        apply_prop(&prop(
            &source,
            &target,
            "missing",
            PropType::String,
            brul_codegen::runtime::PropExpression::Text(text!("colour")),
        ));
        assert!(target.borrow().read("missing").is_err());
    }

    fn command_host(app: Option<Box<dyn Host>>) -> (CommandHost, Rc<RefCell<Vec<WindowCommand>>>) {
        let commands = Rc::new(RefCell::new(Vec::new()));
        (
            CommandHost {
                commands: Rc::clone(&commands),
                app,
                windows: Rc::new(HashMap::new()),
                theme: Rc::new(RefCell::new(None)),
            },
            commands,
        )
    }

    #[test]
    fn theme_reports_the_shown_theme() {
        let (mut host, _) = command_host(None);
        assert_eq!(host.call("theme", &[]), Ok(Value::Null));

        *host.theme.borrow_mut() = Some("dark".to_owned());
        assert_eq!(
            host.call("theme", &[]),
            Ok(Value::String("dark".to_owned().into()))
        );
        assert_eq!(
            host.call("theme", &[Value::String("dark".into())]),
            Err(Error::Arity {
                name: "theme".to_owned(),
                expected: "no",
            })
        );
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
            host.call("setTheme", &[]),
            Err(Error::Arity {
                name: "setTheme".to_owned(),
                expected: "one",
            })
        );
        assert_eq!(
            host.call("setTheme", &[Value::Number(1.0)]),
            Err(Error::Argument {
                name: "setTheme",
                expected: "String",
                found: "Number",
            })
        );
        assert_eq!(
            host.call("setTheme", &[Value::String("dark".into())]),
            Ok(Value::Null)
        );
        assert_eq!(
            *commands.borrow(),
            vec![WindowCommand::SetTheme("dark".to_owned())]
        );
    }

    #[test]
    fn set_always_on_top_takes_exactly_one_flag() {
        let (mut host, commands) = command_host(None);
        assert_eq!(
            host.call("setAlwaysOnTop", &[]),
            Err(Error::Arity {
                name: "setAlwaysOnTop".to_owned(),
                expected: "one",
            })
        );
        assert_eq!(
            host.call("setAlwaysOnTop", &[Value::Number(1.0)]),
            Err(Error::Argument {
                name: "setAlwaysOnTop",
                expected: "Bool",
                found: "Number",
            })
        );
        assert_eq!(
            host.call("setAlwaysOnTop", &[Value::Bool(true)]),
            Ok(Value::Null)
        );
        assert_eq!(*commands.borrow(), vec![WindowCommand::AlwaysOnTop(true)]);
    }

    #[test]
    fn every_builtin_is_listed_by_name() {
        let (mut host, _) = command_host(None);
        assert_eq!(host.commands(), BUILTINS.to_vec());
        for name in BUILTINS {
            assert!(
                !matches!(host.call(name, &[]), Err(Error::UnknownFunction(_))),
                "{name} is dispatched but not listed"
            );
        }
    }

    #[test]
    fn the_builtin_commands_take_no_arguments() {
        let (mut host, commands) = command_host(None);
        assert_eq!(host.call("exit", &[]), Ok(Value::Null));
        assert_eq!(host.call("closeWindow", &[]), Ok(Value::Null));
        assert_eq!(host.call("dragWindow", &[]), Ok(Value::Null));
        assert_eq!(host.call("showWindow", &[]), Ok(Value::Null));
        assert_eq!(host.call("hideWindow", &[]), Ok(Value::Null));
        assert_eq!(host.call("focusWindow", &[]), Ok(Value::Null));
        assert_eq!(
            host.call("exit", &[Value::Number(1.0)]),
            Err(Error::Arity {
                name: "exit".to_owned(),
                expected: "no",
            })
        );
        assert_eq!(
            *commands.borrow(),
            vec![
                WindowCommand::Close,
                WindowCommand::Close,
                WindowCommand::Drag,
                WindowCommand::Show,
                WindowCommand::Hide,
                WindowCommand::Focus
            ]
        );
    }

    #[test]
    fn open_window_finds_a_declared_name() {
        let commands = Rc::new(RefCell::new(Vec::new()));
        let mut windows = HashMap::new();
        windows.insert("Settings".to_owned(), WindowKey(1));
        let mut host = CommandHost {
            commands: Rc::clone(&commands),
            app: None,
            windows: Rc::new(windows),
            theme: Rc::new(RefCell::new(None)),
        };

        assert_eq!(
            host.call("openWindow", &[Value::String("Settings".into())]),
            Ok(Value::Null)
        );
        assert_eq!(
            host.call("openWindow", &[Value::String("Missing".into())]),
            Err(Error::UnknownWindow("Missing".to_owned()))
        );
        assert_eq!(
            host.call("openWindow", &[]),
            Err(Error::Arity {
                name: "openWindow".to_owned(),
                expected: "one",
            })
        );
        assert_eq!(*commands.borrow(), vec![WindowCommand::Open(WindowKey(1))]);
    }

    #[test]
    fn an_unknown_command_without_a_host_is_an_error() {
        let (mut host, _commands) = command_host(None);
        assert_eq!(
            host.call("appCommand", &[]),
            Err(Error::UnknownFunction("appCommand".to_owned()))
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
        assert_eq!(host.call("appCommand", &[]), Ok(Value::Null));
        assert_eq!(host.every(16.0, "tick"), Ok(()));
        assert_eq!(
            *calls.borrow(),
            vec!["appCommand".to_owned(), "tick@16".to_owned()]
        );
    }

    fn themed_view(
        theme: Option<TestTheme>,
        _build: &mut Build,
        _component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        Ok(Built {
            widget: Widget::text(theme.map(Theme::name).unwrap_or("none")),
            bindings: Vec::new(),
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    fn rejecting_view(
        theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        themed_view(None, &mut Build::new(), component)
    }

    #[test]
    fn a_theme_change_rebuilds_the_view() {
        let (component, built, structures) =
            generated_built(themed_view, NAMES, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
        ui.set_theme(TestTheme::Dark).expect("theme changes");
        assert_eq!(ui.theme, Some(TestTheme::Dark));
    }

    #[test]
    fn a_failed_theme_change_keeps_the_previous_theme() {
        let (component, built, structures) =
            generated_built(rejecting_view, NAMES, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(
            component,
            built,
            rejecting_view,
            Some(TestTheme::Light),
            structures,
            None,
        );
        ui.set_theme(TestTheme::Light)
            .expect("same theme is a no-op");
        assert!(ui.set_theme(TestTheme::Dark).is_err());
        assert_eq!(ui.theme, Some(TestTheme::Light));
    }

    fn binding(component: &ComponentHandle, widget: WidgetId, target: BindingTarget) -> Binding {
        Binding::new(component, widget, target)
    }

    fn icon(component: &mut Component) -> Result<Dyn, Error> {
        Ok(Dyn::from(component.read("icon")?))
    }

    fn image_view(
        theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let widget = Widget::image(Texture::empty());
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![binding(component, id, BindingTarget::Image(icon))],
            handlers: Vec::new(),
            props: Vec::new(),
        })
    }

    #[test]
    fn an_image_binding_reaches_the_tree_through_ui() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/crates/brul-gui/tests/fixtures/tint.png"
        );
        let (component, built, structures) = generated_built(
            image_view,
            NAMES,
            None,
            &[("icon", Value::Image(path.into()))],
            None,
        )
        .expect("launch");
        let ui = Ui::assemble(component, built, image_view, None, structures, None);
        let mut engine = TextEngine::new();
        let mut tree = ui.tree.borrow_mut();

        tree.layout(
            &mut engine,
            Size {
                width: 100,
                height: 100,
            },
            1.0,
        )
        .expect("layout");
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.nodes.first().expect("node");

        assert!(node.width > 1);
    }

    fn state_after(targets: [BindingTarget; 6]) -> (Vec<WidgetId>, WidgetTree) {
        let mut component = Component::new();
        component.set_names(NAMES);
        component
            .declare("title", Value::String("hi".into()), false)
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
        let mut tree = WidgetTree::new(Widget::view(Direction::Column, widgets));
        for (index, target) in targets.into_iter().enumerate() {
            let binding = binding(&component, ids[index], target);
            let pending = runtime::evaluate(&binding, &runtime::Textures::default());
            runtime::write(pending, binding.widget, &mut tree);
        }
        (ids, tree)
    }

    #[test]
    fn every_binding_target_writes_its_own_state() {
        let targets = [
            BindingTarget::Text(text!("title")),
            BindingTarget::Checked(flag!("on")),
            BindingTarget::Selected(flag!("on")),
            BindingTarget::Disabled(flag!("on")),
            BindingTarget::Visible(flag!("off")),
            BindingTarget::Offset {
                x: Some(number!("x")),
                y: Some(number!("y")),
            },
        ];
        let (ids, mut tree) = state_after(targets);
        assert!(!tree.set_text(ids[0], "hi".to_owned()));
        let state = tree.state(ids[1]).expect("state");
        assert!(state.checked);
        assert!(tree.state(ids[2]).expect("state").selected);
        assert!(tree.state(ids[3]).expect("state").disabled);
        assert!(!tree.state(ids[4]).expect("state").visible);
        let mut engine = TextEngine::new();
        tree.layout(
            &mut engine,
            Size {
                width: 200,
                height: 200,
            },
            1.0,
        )
        .expect("layout");
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(ids[5]).cloned().expect("node");
        assert_eq!((node.x, node.y), (4, 6));
    }

    fn handler_dispatch(
        component: &mut Component,
        name: &str,
        arguments: Vec<Dyn>,
    ) -> Result<Dyn, Error> {
        match name {
            "clicked" | "closing" | "tick" | "first" => {
                component.assign("ran", Value::Bool(true))?;
            }
            "second" => component.assign("off", Value::Bool(true))?,
            "pressed" => component.assign("seen", Value::from(arguments[0].clone()))?,
            _ => return Err(Error::UnknownFunction(name.to_owned())),
        }
        Ok(Dyn::null())
    }

    fn component_with(declared: &[&str]) -> ComponentHandle {
        let mut component = Component::new();
        component.set_names(NAMES);
        for name in declared {
            component
                .declare(name, Value::Null, false)
                .expect("declared");
        }
        component.set_dispatch(handler_dispatch);
        component.register_event_call("tick", set_ran);
        component.register_event_call("applied", applied_call);
        Rc::new(RefCell::new(component))
    }

    fn noop_init(_: &mut Component) -> Result<(), Error> {
        Ok(())
    }

    fn noop_dispatch(_: &mut Component, _: &str, _: Vec<Dyn>) -> Result<Dyn, Error> {
        Ok(Dyn::null())
    }

    fn raise_dispatch(
        component: &mut Component,
        name: &str,
        arguments: Vec<Dyn>,
    ) -> Result<Dyn, Error> {
        if name == "raise" {
            let arguments = arguments.into_iter().map(Value::from).collect();
            Ok(Dyn::from(crate::eval::call(
                component, "clicked", arguments,
            )?))
        } else {
            Err(Error::UnknownFunction(name.to_owned()))
        }
    }

    fn applied_dispatch(component: &mut Component, name: &str, _: Vec<Dyn>) -> Result<Dyn, Error> {
        if name == "applied" {
            component.assign("label", Value::String("after".into()))?;
            Ok(Dyn::null())
        } else {
            Err(Error::UnknownFunction(name.to_owned()))
        }
    }

    fn set_ran(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
        component.assign("ran", Value::Bool(true))?;
        Ok(Dyn::null())
    }

    fn set_off(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
        component.assign("off", Value::Bool(true))?;
        Ok(Dyn::null())
    }

    fn key_code(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
        component.assign("seen", Value::from(arguments[0].clone()))?;
        Ok(Dyn::null())
    }

    fn raise_call(component: &mut Component, arguments: &[Dyn]) -> Result<Dyn, Error> {
        raise_dispatch(component, "raise", arguments.to_vec())
    }

    fn applied_call(component: &mut Component, _: &[Dyn]) -> Result<Dyn, Error> {
        component.assign("label", Value::String("after".into()))?;
        Ok(Dyn::null())
    }

    fn script_with(init: fn(&mut Component) -> Result<(), Error>) -> Script {
        Script::Generated {
            init,
            dispatch: noop_dispatch,
        }
    }

    fn shown_false(component: &mut Component) -> Result<(), Error> {
        component.declare("shown", Value::Bool(false), false)
    }

    fn shown_and_label(component: &mut Component) -> Result<(), Error> {
        component.declare("shown", Value::Bool(false), false)?;
        component.declare("label", Value::String("bound".into()), false)
    }

    fn dispatching(
        component: &ComponentHandle,
        widget: Widget,
        handlers: impl FnOnce(WidgetId) -> Vec<Handler>,
    ) -> (WidgetId, Ui<TestTheme>) {
        let id = widget.id();
        let built = Built {
            widget,
            bindings: Vec::new(),
            handlers: handlers(id),
            props: Vec::new(),
        };
        (
            id,
            Ui::assemble(
                Rc::clone(component),
                built,
                themed_view,
                None,
                Vec::new(),
                None,
            ),
        )
    }

    #[test]
    fn dispatch_calls_the_handler_of_the_widget() {
        let component = component_with(&["ran"]);
        let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
            vec![Handler::new(
                &component,
                Some(id),
                HandlerEvent::Click,
                set_ran,
            )]
        });
        ui.dispatch(&[WidgetEvent::Click(id)]);
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    }

    #[test]
    fn dispatch_runs_the_first_of_two_handlers_of_a_widget() {
        let component = component_with(&["ran", "off"]);
        let (id, mut ui) = dispatching(&component, Widget::text(""), |id| {
            vec![
                Handler::new(&component, Some(id), HandlerEvent::Click, set_ran),
                Handler::new(&component, Some(id), HandlerEvent::Click, set_off),
            ]
        });

        ui.dispatch(&[WidgetEvent::Click(id)]);
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
        assert_ne!(ui.component.borrow().read("off"), Ok(Value::Bool(true)));
    }

    #[test]
    fn dispatch_passes_the_key_code() {
        let component = component_with(&["seen"]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
            vec![Handler::new(&component, None, HandlerEvent::Key, key_code)]
        });
        ui.dispatch(&[WidgetEvent::Key {
            code: "Enter".to_owned(),
            modifiers: brul_gui::KeyModifiers::default(),
        }]);
        let seen = ui.component.borrow().read("seen").expect("seen");
        assert_eq!(
            seen,
            Value::Object(
                vec![
                    ("code".to_owned(), Value::String("Enter".into())),
                    ("ctrl".to_owned(), Value::Bool(false)),
                    ("shift".to_owned(), Value::Bool(false)),
                    ("alt".to_owned(), Value::Bool(false)),
                    ("meta".to_owned(), Value::Bool(false)),
                ]
                .into()
            )
        );
    }

    #[test]
    fn a_close_handler_keeps_the_window_open() {
        let component = component_with(&["ran"]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| {
            vec![Handler::new(&component, None, HandlerEvent::Close, set_ran)]
        });

        assert!(ui.dispatch(&[WidgetEvent::Close]).is_empty());
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    }

    #[test]
    fn a_close_without_a_handler_closes_the_window() {
        let component = component_with(&[]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());

        assert_eq!(
            ui.dispatch(&[WidgetEvent::Close]),
            vec![WindowCommand::Close]
        );
    }

    #[test]
    fn dispatch_runs_a_timer() {
        let component = component_with(&["ran"]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
        ui.dispatch(&[WidgetEvent::Timer("tick".into())]);
        assert_eq!(ui.component.borrow().read("ran"), Ok(Value::Bool(true)));
    }

    #[test]
    fn dispatch_ignores_events_without_a_handler() {
        let component = component_with(&[]);
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
        let component = component_with(&[]);
        let (_, mut ui) = dispatching(&component, Widget::text(""), |_| Vec::new());
        ui.commands.borrow_mut().push(WindowCommand::Exit);
        assert_eq!(ui.dispatch(&[]), vec![WindowCommand::Exit]);
    }

    #[test]
    fn dispatch_returns_the_theme_command() {
        let (component, built, structures) =
            generated_built(themed_view, NAMES, None, &[], None).expect("launch");
        let mut ui = Ui::assemble(component, built, themed_view, None, structures, None);
        ui.commands
            .borrow_mut()
            .push(WindowCommand::SetTheme("dark".to_owned()));
        assert_eq!(
            ui.dispatch(&[]),
            vec![WindowCommand::SetTheme("dark".to_owned())]
        );
        assert!(ui.theme.is_none());
    }

    fn child_view(
        theme: Option<&str>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let text = Widget::text("child");
        let id = text.id();
        Ok(Built {
            widget: text,
            bindings: Vec::new(),
            handlers: vec![Handler::new(
                component,
                Some(id),
                HandlerEvent::Click,
                raise_call,
            )],
            props: Vec::new(),
        })
    }

    fn raising_view(
        theme: Option<TestTheme>,
        _build: &mut Build,
        component: &ComponentHandle,
    ) -> Result<Built, ViewError> {
        if theme.is_some() {
            return Err(unknown_theme());
        }
        let script = Script::Generated {
            init: noop_init,
            dispatch: raise_dispatch,
        };
        let events = [runtime::ComponentEvent {
            name: "clicked",
            call: applied_call,
            params: &[],
            component: Rc::clone(component),
        }];
        let child = runtime::instance(
            &mut Build::new(),
            component,
            runtime::Instance {
                theme: theme.map(Theme::name),
                view: child_view,
                script: Some(script),
                name: "Child",
                names: NAMES,
                props: &[],
                defaults: &[],
                events: &events,
                slots: &[],
            },
        )?;
        let text = Widget::text("");
        let id = text.id();
        Ok(Built {
            widget: Widget::view(Direction::Column, vec![child.widget, text]),
            bindings: vec![Binding::new(
                component,
                id,
                BindingTarget::Text(text!("label")),
            )],
            handlers: child.handlers,
            props: Vec::new(),
        })
    }

    #[test]
    fn a_handler_event_runs_after_the_handler_returns() {
        let (component, built, structures) =
            generated_built(raising_view, NAMES, None, &[], None).expect("launch");
        component
            .borrow_mut()
            .declare("label", Value::String("".into()), false)
            .expect("declared");
        let click = built.handlers[0].widget.expect("widget");
        let text = built.bindings[0].widget;
        component.borrow_mut().set_dispatch(applied_dispatch);
        let mut ui = Ui::assemble(component, built, raising_view, None, structures, None);

        ui.dispatch(&[WidgetEvent::Click(click)]);

        assert_eq!(
            ui.component.borrow().read("label"),
            Ok(Value::String("after".into()))
        );
        assert!(!ui.tree.borrow_mut().set_text(text, "after".to_owned()));
    }
}

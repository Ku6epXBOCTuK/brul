include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
}

impl ThemeId {
    pub const ALL: &'static [ThemeId] = &[
    ];
}

impl brul::Theme for ThemeId {
    fn name(self) -> &'static str {
        match self {}
    }

    fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|theme| brul::Theme::name(*theme) == name)
    }
}

impl std::str::FromStr for ThemeId {
    type Err = brul::UnknownTheme;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        <Self as brul::Theme>::parse(name).ok_or_else(|| brul::UnknownTheme::new(name))
    }
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("demo", "attach:editor"),
        component: std::rc::Rc::clone(component),
        params: vec![],
        apply: ::brul::policy::__brul_attach_editor_apply,
        event: ::brul::policy::__brul_attach_editor_event,
        raw_event: ::brul::policy::__brul_attach_editor_raw_event,
        cleanup: ::brul::policy::__brul_attach_editor_cleanup,
        events: brul::EventSet::from([]),
        raw_events: brul::RawEventSet::from([brul::RawKind::Key, brul::RawKind::Ime, brul::RawKind::Paste]),
    }));
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("demo", "attach:keyboard"),
        component: std::rc::Rc::clone(component),
        params: vec![],
        apply: ::brul::policy::__brul_attach_keyboard_apply,
        event: ::brul::policy::__brul_attach_keyboard_event,
        raw_event: ::brul::policy::__brul_attach_keyboard_raw_event,
        cleanup: ::brul::policy::__brul_attach_keyboard_cleanup,
        events: brul::EventSet::from([]),
        raw_events: brul::RawEventSet::from([brul::RawKind::Key]),
    }));
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("demo", "attach:pointer"),
        component: std::rc::Rc::clone(component),
        params: vec![],
        apply: ::brul::policy::__brul_attach_pointer_apply,
        event: ::brul::policy::__brul_attach_pointer_event,
        raw_event: ::brul::policy::__brul_attach_pointer_raw_event,
        cleanup: ::brul::policy::__brul_attach_pointer_cleanup,
        events: brul::EventSet::from([]),
        raw_events: brul::RawEventSet::from([brul::RawKind::PointerEnter, brul::RawKind::PointerLeave, brul::RawKind::PointerCancel, brul::RawKind::PointerMove, brul::RawKind::Press, brul::RawKind::Release, brul::RawKind::Drag]),
    }));
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("demo", "attach:scroll"),
        component: std::rc::Rc::clone(component),
        params: vec![],
        apply: ::brul::policy::__brul_attach_scroll_apply,
        event: ::brul::policy::__brul_attach_scroll_event,
        raw_event: ::brul::policy::__brul_attach_scroll_raw_event,
        cleanup: ::brul::policy::__brul_attach_scroll_cleanup,
        events: brul::EventSet::from([]),
        raw_events: brul::RawEventSet::from([brul::RawKind::Wheel]),
    }));
    let widget = {
        brul::gui::Widget::view(brul::gui::Direction::Row, vec![
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: input_view,
                    script: input_script(),
                    name: "Input",
                    names: names(),
                    props: &[
                        brul::PropArgument { name: "value", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok(component.state::<DemoScriptState>().draft.get()) }) },
                    ],
                    defaults: &[
                    ],
                    events: &[
                        brul::ComponentEvent { name: "input", call: demo_script_event_edited, params: &[brul::PropType::Object], component: std::rc::Rc::clone(component) },
                        brul::ComponentEvent { name: "key", call: demo_script_event_pressed, params: &[brul::PropType::Object], component: std::rc::Rc::clone(component) },
                    ],
                    slots: &[],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
                widget0
            },
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: input_view,
                    script: input_script(),
                    name: "Input",
                    names: names(),
                    props: &[
                    ],
                    defaults: &[
                        brul::PropArgument { name: "value", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("".to_owned()) }) },
                    ],
                    events: &[
                    ],
                    slots: &[],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget1 = built.widget;
                widget1
            },
        ])
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn demo_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*DEMO_CLASSES_NONE
}

#[allow(dead_code)]
fn demo_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![
            s::ClassRule {
                class: "brul-input".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("text".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "brul-draggable".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("grab".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "brul-draggable".into(),
                state: Some(s::State::Pressed),
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("grabbing".into()) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn demo_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = demo_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

#[allow(dead_code)]
pub struct DemoScriptState {
    pub draft: brul::Signal<String>,
}

#[allow(dead_code)]
fn demo_script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let draft: String = component.seed("draft", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(DemoScriptState { draft: brul::Signal::new(draft) });
    Ok(())
}

pub fn demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: demo_script_init, call: demo_script_typed_call, dispatch: demo_script_dispatch })
}
#[allow(unused_variables)]
fn demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("edited", demo_script_event_edited);
    component.register_event_call("pressed", demo_script_event_pressed);
    component.declare("draft", (brul::Dyn::text("".to_owned())).into(), false)?;
    demo_script_state_init(component)?;
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn demo_script_fn_edited(component: &mut brul::Component, mut ev: brul::Dyn) -> Result<(), brul::Error> {
    { let value = (ev.clone()).field("value")?;
    component.state::<DemoScriptState>().draft.set((&value).try_into()?);

value };
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn demo_script_fn_pressed(component: &mut brul::Component, mut ev: brul::Dyn) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_variables)]
fn demo_script_event_edited(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    demo_script_fn_edited(component, (arguments[0]).clone())?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn demo_script_event_pressed(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    demo_script_fn_pressed(component, (arguments[0]).clone())?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum DemoScriptMessage {
    Edited(brul::Dyn),
    Pressed(brul::Dyn),
}
impl From<DemoScriptMessage> for brul::Message {
    fn from(message: DemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<DemoScriptMessage>() {
        Some(DemoScriptMessage::Edited(argument0)) => { demo_script_fn_edited(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        Some(DemoScriptMessage::Pressed(argument0)) => { demo_script_fn_pressed(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "edited" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            demo_script_fn_edited(component, arguments.remove(0))?;
            Ok(brul::Dyn::null())
        }
        "pressed" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            demo_script_fn_pressed(component, arguments.remove(0))?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct DemoScript(brul::Handle);

#[allow(dead_code)]
impl DemoScript {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }

    pub fn edited(&self, ev: brul::Dyn) {
        self.0.send(DemoScriptMessage::Edited(ev));
    }

    pub fn pressed(&self, ev: brul::Dyn) {
        self.0.send(DemoScriptMessage::Pressed(ev));
    }
}

#[allow(dead_code)]
pub type Script = DemoScript;

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn input_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = input_styled(brul::gui::Widget::input(""), &[0, 1], theme);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Input(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(2))?)).map_err(|error| error.at(brul::ast::Position { line: 17, column: 69 }))?; value }) })));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::EventKind::Input, input_script_event_changed));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::EventKind::Key, input_script_event_typed));
            widget0
        }
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static INPUT_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 2]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { cursor: Some(brul::gui::Cursor::Text), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn input_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*INPUT_CLASSES_NONE
}

#[allow(dead_code)]
fn input_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![
            s::ClassRule {
                class: "brul-input".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("text".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "brul-draggable".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("grab".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "brul-draggable".into(),
                state: Some(s::State::Pressed),
                properties: vec![
                    s::Property { kind: s::Kind::Cursor, value: s::Value::Keyword("grabbing".into()) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn input_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = input_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn input_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: input_script_init, call: input_script_typed_call, dispatch: input_script_dispatch })
}
#[allow(unused_variables)]
fn input_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("changed", input_script_event_changed);
    component.register_event_call("typed", input_script_event_typed);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn input_script_fn_changed(component: &mut brul::Component, mut ev: brul::Dyn) -> Result<(), brul::Error> {
    { let arguments = vec![ev.clone()]; brul::Dyn::call(component, "input", arguments)? };
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn input_script_fn_typed(component: &mut brul::Component, mut ev: brul::Dyn) -> Result<(), brul::Error> {
    { let arguments = vec![ev.clone()]; brul::Dyn::call(component, "key", arguments)? };
    Ok(())
}
#[allow(unused_variables)]
fn input_script_event_changed(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    input_script_fn_changed(component, (arguments[0]).clone())?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn input_script_event_typed(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    input_script_fn_typed(component, (arguments[0]).clone())?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum InputScriptMessage {
    Changed(brul::Dyn),
    Typed(brul::Dyn),
}
impl From<InputScriptMessage> for brul::Message {
    fn from(message: InputScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn input_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<InputScriptMessage>() {
        Some(InputScriptMessage::Changed(argument0)) => { input_script_fn_changed(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        Some(InputScriptMessage::Typed(argument0)) => { input_script_fn_typed(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn input_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "changed" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            input_script_fn_changed(component, arguments.remove(0))?;
            Ok(brul::Dyn::null())
        }
        "typed" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            input_script_fn_typed(component, arguments.remove(0))?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn demo_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "demo".to_owned(),
        width: 960,
        height: 640,
        frameless: false,
        transparent: false,
        visible: true,
        centered: false,
        scroll: true,
        position: None,
        icon: None,
        effects: __brul_effects(),
        ..brul::WindowConfig::default()
    }
}

pub fn windows() -> Vec<brul::WindowDef<ThemeId>> {
    #[cfg(brul_alloc_count)]
    brul::perf::measure(brul_allocations::reset, brul_allocations::report);
    vec![
        brul::WindowDef { name: "Demo", config: demo_window(), view: demo_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["draft", "ev", "value"]
}

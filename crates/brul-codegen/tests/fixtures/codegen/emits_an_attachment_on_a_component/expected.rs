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
pub fn editor_host_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("editor_host", "attach:editor"),
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
        state: brul::StateId::source("editor_host", "attach:keyboard"),
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
        state: brul::StateId::source("editor_host", "attach:pointer"),
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
        state: brul::StateId::source("editor_host", "attach:scroll"),
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
        {
            let built = brul::instance(build, component, brul::Instance {
                theme,
                view: editor_view,
                script: editor_script(),
                name: "Editor",
                names: names(),
                props: &[
                ],
                defaults: &[
                ],
                events: &[
                ],
                slots: &[],
            })?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget0 = built.widget;
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
static EDITOR_HOST_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn editor_host_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*EDITOR_HOST_CLASSES_NONE
}

#[allow(dead_code)]
fn editor_host_styles() -> &'static brul::style::Stylesheet {
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

fn editor_host_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = editor_host_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn editor_host_script() -> Option<brul::Script> {
    None
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn editor_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = editor_styled(brul::gui::Widget::input(""), &[0], theme);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Input(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok(component.state::<EditorScriptState>().draft.get()) })));
            let _ = build.capability(brul::Capability::hooks(brul::Hooks {
                target: brul::EventTarget::Widget(widget0.id()),
                state: brul::StateId::source("editor", "TextInput/attach:custom_editor"),
                component: std::rc::Rc::clone(component),
                params: vec![|component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> { let value = brul::Dyn::text(component.state::<EditorScriptState>().draft.get()); Ok(value) }],
                apply: crate::__brul_attach_custom_editor_apply,
                event: crate::__brul_attach_custom_editor_event,
                raw_event: crate::__brul_attach_custom_editor_raw_event,
                cleanup: crate::__brul_attach_custom_editor_cleanup,
                events: brul::EventSet::from([brul::EventKind::Key]),
                raw_events: brul::RawEventSet::from([]),
            }));
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
static EDITOR_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { cursor: Some(brul::gui::Cursor::Text), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn editor_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*EDITOR_CLASSES_NONE
}

#[allow(dead_code)]
fn editor_styles() -> &'static brul::style::Stylesheet {
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

fn editor_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = editor_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

#[allow(dead_code)]
pub struct EditorScriptState {
    pub draft: brul::Signal<String>,
}

#[allow(dead_code)]
fn editor_script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let draft: String = component.seed("draft", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(EditorScriptState { draft: brul::Signal::new(draft) });
    Ok(())
}

pub fn editor_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: editor_script_init, call: editor_script_typed_call, dispatch: editor_script_dispatch })
}
#[allow(unused_variables)]
fn editor_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.declare("draft", (brul::Dyn::text("".to_owned())).into(), false)?;
    editor_script_state_init(component)?;
    Ok(())
}

#[allow(dead_code)]
pub enum EditorScriptMessage {
}
impl From<EditorScriptMessage> for brul::Message {
    fn from(message: EditorScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn editor_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<EditorScriptMessage>() {
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn editor_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn editor_host_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "editor".to_owned(),
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
        brul::WindowDef { name: "EditorHost", config: editor_host_window(), view: editor_host_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    editor_host_script()
}

pub fn names() -> &'static [&'static str] {
    &["draft"]
}

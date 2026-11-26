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
pub fn web_view_demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("web_view_demo", "attach:editor"),
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
        state: brul::StateId::source("web_view_demo", "attach:keyboard"),
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
        state: brul::StateId::source("web_view_demo", "attach:pointer"),
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
        state: brul::StateId::source("web_view_demo", "attach:scroll"),
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
            let built = build.capability(brul::Capability::view(brul::StateId::source("web_view_demo", "WebView/view"), std::rc::Rc::clone(component), brul::Instance {
                theme,
                view: ::brul::webview::view,
                script: None,
                name: "WebView",
                names: names(),
                props: &[
                    brul::PropArgument { name: "source", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("https://example.com".to_owned()) }) },
                    brul::PropArgument { name: "visible", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(true) }) },
                ],
                defaults: &[],
                events: &[
                    brul::ComponentEvent { name: "message", call: web_view_demo_script_event_received, params: &[brul::PropType::String], component: std::rc::Rc::clone(component) },
                ],
                slots: &[],
            }))?.expect("view capability");
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget0 = built.widget.debug_named("WebView");
            widget0
        }.debug_source("input.brul")
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static WEB_VIEW_DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn web_view_demo_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*WEB_VIEW_DEMO_CLASSES_NONE
}

#[allow(dead_code)]
fn web_view_demo_styles() -> &'static brul::style::Stylesheet {
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

fn web_view_demo_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = web_view_demo_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn web_view_demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: web_view_demo_script_init, call: web_view_demo_script_typed_call, dispatch: web_view_demo_script_dispatch })
}
#[allow(unused_variables)]
fn web_view_demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("received", web_view_demo_script_event_received);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn web_view_demo_script_fn_received(component: &mut brul::Component, mut message: String) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_variables)]
fn web_view_demo_script_event_received(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    web_view_demo_script_fn_received(component, (&arguments[0]).try_into()?)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum WebViewDemoScriptMessage {
    Received(String),
}
impl From<WebViewDemoScriptMessage> for brul::Message {
    fn from(message: WebViewDemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn web_view_demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<WebViewDemoScriptMessage>() {
        Some(WebViewDemoScriptMessage::Received(argument0)) => { web_view_demo_script_fn_received(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn web_view_demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "received" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            web_view_demo_script_fn_received(component, (&arguments.remove(0)).try_into()?)?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct WebViewDemoScript(brul::Handle);

#[allow(dead_code)]
impl WebViewDemoScript {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }

    pub fn received(&self, message: String) {
        self.0.send(WebViewDemoScriptMessage::Received(message));
    }
}

#[allow(dead_code)]
pub type Script = WebViewDemoScript;

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn web_view_demo_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "webview".to_owned(),
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
        brul::WindowDef { name: "WebViewDemo", config: web_view_demo_window(), view: web_view_demo_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    web_view_demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["message", "source", "visible"]
}

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
pub fn video_demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let built = build.capability(brul::Capability::view(brul::StateId::source("video_demo", "Video/view"), std::rc::Rc::clone(component), brul::Instance {
                theme,
                view: ::brul::video::view,
                script: None,
                name: "Video",
                names: names(),
                props: &[
                    brul::PropArgument { name: "source", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("clip.gif".to_owned()) }) },
                    brul::PropArgument { name: "playing", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(true) }) },
                ],
                defaults: &[],
                events: &[
                    brul::ComponentEvent { name: "ended", call: video_demo_script_event_finished, params: &[], component: std::rc::Rc::clone(component) },
                    brul::ComponentEvent { name: "error", call: video_demo_script_event_failed, params: &[brul::PropType::String], component: std::rc::Rc::clone(component) },
                ],
                slots: &[],
            }))?.expect("view capability");
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
static VIDEO_DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn video_demo_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*VIDEO_DEMO_CLASSES_NONE
}

#[allow(dead_code)]
fn video_demo_styles() -> &'static brul::style::Stylesheet {
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

fn video_demo_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = video_demo_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn video_demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: video_demo_script_init, call: video_demo_script_typed_call, dispatch: video_demo_script_dispatch })
}
#[allow(unused_variables)]
fn video_demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("finished", video_demo_script_event_finished);
    component.register_event_call("failed", video_demo_script_event_failed);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn video_demo_script_fn_finished(component: &mut brul::Component) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn video_demo_script_fn_failed(component: &mut brul::Component, mut message: String) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_variables)]
fn video_demo_script_event_finished(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    video_demo_script_fn_finished(component)?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn video_demo_script_event_failed(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    video_demo_script_fn_failed(component, (&arguments[0]).try_into()?)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum VideoDemoScriptMessage {
    Finished,
    Failed(String),
}
impl From<VideoDemoScriptMessage> for brul::Message {
    fn from(message: VideoDemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn video_demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<VideoDemoScriptMessage>() {
        Some(VideoDemoScriptMessage::Finished) => { video_demo_script_fn_finished(component)?; Ok(brul::Dyn::null()) }
        Some(VideoDemoScriptMessage::Failed(argument0)) => { video_demo_script_fn_failed(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn video_demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "finished" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            video_demo_script_fn_finished(component, )?;
            Ok(brul::Dyn::null())
        }
        "failed" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            video_demo_script_fn_failed(component, (&arguments.remove(0)).try_into()?)?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct VideoDemoScript(brul::Handle);

#[allow(dead_code)]
impl VideoDemoScript {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }

    pub fn finished(&self) {
        self.0.send(VideoDemoScriptMessage::Finished);
    }

    pub fn failed(&self, message: String) {
        self.0.send(VideoDemoScriptMessage::Failed(message));
    }
}

#[allow(dead_code)]
pub type Script = VideoDemoScript;

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn video_demo_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "video".to_owned(),
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
        brul::WindowDef { name: "VideoDemo", config: video_demo_window(), view: video_demo_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    video_demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["message", "source", "playing"]
}

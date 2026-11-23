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
pub fn gauge_demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let built = build.capability(brul::Capability::view(brul::StateId::new(4743421592828437u64), std::rc::Rc::clone(component), brul::Instance {
                theme,
                view: crate::__brul_widget_gauge,
                script: None,
                name: "Gauge",
                names: names(),
                props: &[
                    brul::PropArgument { name: "value", ty: brul::PropType::Number, expression: brul::PropExpression::Number(|component: &mut brul::Component| -> Result<f64, brul::Error> { Ok(42.0_f64) }) },
                    brul::PropArgument { name: "label", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("load".to_owned()) }) },
                ],
                defaults: &[],
                events: &[
                    brul::ComponentEvent { name: "changed", call: gauge_demo_script_event_changed, params: &[brul::PropType::Number], component: std::rc::Rc::clone(component) },
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
static GAUGE_DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn gauge_demo_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*GAUGE_DEMO_CLASSES_NONE
}

#[allow(dead_code)]
fn gauge_demo_styles() -> &'static brul::style::Stylesheet {
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

fn gauge_demo_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = gauge_demo_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn gauge_demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: gauge_demo_script_init, call: gauge_demo_script_typed_call, dispatch: gauge_demo_script_dispatch })
}
#[allow(unused_variables)]
fn gauge_demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("changed", gauge_demo_script_event_changed);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn gauge_demo_script_fn_changed(component: &mut brul::Component, mut amount: f64) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_variables)]
fn gauge_demo_script_event_changed(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    gauge_demo_script_fn_changed(component, (&arguments[0]).try_into()?)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum GaugeDemoScriptMessage {
    Changed(f64),
}
impl From<GaugeDemoScriptMessage> for brul::Message {
    fn from(message: GaugeDemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn gauge_demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<GaugeDemoScriptMessage>() {
        Some(GaugeDemoScriptMessage::Changed(argument0)) => { gauge_demo_script_fn_changed(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn gauge_demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "changed" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            gauge_demo_script_fn_changed(component, (&arguments.remove(0)).try_into()?)?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct GaugeDemoScript(brul::Handle);

#[allow(dead_code)]
impl GaugeDemoScript {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }

    pub fn changed(&self, amount: f64) {
        self.0.send(GaugeDemoScriptMessage::Changed(amount));
    }
}

#[allow(dead_code)]
pub type Script = GaugeDemoScript;

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn gauge_demo_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "gauge".to_owned(),
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
        brul::WindowDef { name: "GaugeDemo", config: gauge_demo_window(), view: gauge_demo_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    gauge_demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["amount", "value", "label"]
}

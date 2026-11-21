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
    let widget = {
        brul::gui::Widget::view(brul::gui::Direction::Row, vec![])
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
        rules: vec![],
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

pub fn demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: demo_script_init, call: demo_script_typed_call, dispatch: demo_script_dispatch })
}
#[allow(unused_variables)]
fn demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("double", demo_script_event_double);
    component.register_event_call("four", demo_script_event_four);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn demo_script_fn_double(component: &mut brul::Component, mut value: f64) -> Result<f64, brul::Error> {
    return Ok((value) * (2.0_f64));
    Err(brul::Error::Unsupported("missing return"))
}
#[allow(unused_mut, unused_variables)]
fn demo_script_fn_four(component: &mut brul::Component) -> Result<f64, brul::Error> {
    return Ok(demo_script_fn_double(component, 2.0_f64)?);
    Err(brul::Error::Unsupported("missing return"))
}
#[allow(unused_variables)]
fn demo_script_event_double(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    let result = demo_script_fn_double(component, (&arguments[0]).try_into()?)?;
    Ok(brul::Dyn::num(result))
}
#[allow(unused_variables)]
fn demo_script_event_four(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    let result = demo_script_fn_four(component)?;
    Ok(brul::Dyn::num(result))
}

#[allow(dead_code)]
pub enum DemoScriptMessage {
    Double(f64),
    Four,
}
impl From<DemoScriptMessage> for brul::Message {
    fn from(message: DemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<DemoScriptMessage>() {
        Some(DemoScriptMessage::Double(argument0)) => { let result = demo_script_fn_double(component, (argument0).clone())?; Ok(brul::Dyn::num(result)) }
        Some(DemoScriptMessage::Four) => { let result = demo_script_fn_four(component)?; Ok(brul::Dyn::num(result)) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "double" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            let result = demo_script_fn_double(component, (&arguments.remove(0)).try_into()?)?;
            Ok(brul::Dyn::num(result))
        }
        "four" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            let result = demo_script_fn_four(component, )?;
            Ok(brul::Dyn::num(result))
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

    pub fn double(&self, value: f64) {
        self.0.send(DemoScriptMessage::Double(value));
    }

    pub fn four(&self) {
        self.0.send(DemoScriptMessage::Four);
    }
}

#[allow(dead_code)]
pub type Script = DemoScript;

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
    &["value"]
}

include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        brul::gui::Widget::column(vec![])
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

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

#[allow(dead_code)]
static CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*CLASSES_NONE
}

#[allow(dead_code)]
fn styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![],
    });
    &STYLES
}

fn styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
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

#[allow(dead_code)]
pub struct ScriptState {
    pub total: brul::Signal<f64>,
}

#[allow(dead_code)]
fn script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let total: f64 = component.seed("total", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(ScriptState { total: brul::Signal::new(total) });
    Ok(())
}

pub fn script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: script_init, call: script_typed_call, dispatch: script_dispatch })
}
#[allow(unused_variables)]
fn script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("update", script_event_update);
    component.declare("total", (brul::Dyn::num(0.0_f64)).into(), false)?;
    script_state_init(component)?;
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn script_fn_update(component: &mut brul::Component) -> Result<(), brul::Error> {
    let fixed: f64 = (&brul::Dyn::num(3.0_f64)).try_into()?;
    let mut step: f64 = (&brul::Dyn::num(2.0_f64)).try_into()?;
    if (step) > (1.0_f64) {
    let mut step: f64 = (&brul::Dyn::num(5.0_f64)).try_into()?;
    { let value = brul::Dyn::num((step) + (fixed));
    component.state::<ScriptState>().total.set((&value).try_into()?);

value };
    }
    { let value = brul::Dyn::num((component.state::<ScriptState>().total.get()) + (step));
    component.state::<ScriptState>().total.set((&value).try_into()?);

value };
    Ok(())
}
#[allow(unused_variables)]
fn script_event_update(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    script_fn_update(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum ScriptMessage {
    Update,
}
impl From<ScriptMessage> for brul::Message {
    fn from(message: ScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<ScriptMessage>() {
        Some(ScriptMessage::Update) => { script_fn_update(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "update" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            script_fn_update(component, )?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct Script(brul::Handle);

#[allow(dead_code)]
impl Script {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }

    pub fn update(&self) {
        self.0.send(ScriptMessage::Update);
    }
}

pub fn names() -> &'static [&'static str] {
    &["fixed", "step", "total"]
}

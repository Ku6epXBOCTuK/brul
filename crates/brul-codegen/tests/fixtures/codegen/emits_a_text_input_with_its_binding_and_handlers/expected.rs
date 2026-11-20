include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = styled(brul::gui::Widget::input(""), &[0], theme);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Input(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok((&brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).try_into()?) })));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Input, script_event_edited));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Submit, script_event_submitted));
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
static CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
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

pub fn script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: script_init, call: script_typed_call, dispatch: script_dispatch })
}
#[allow(unused_variables)]
fn script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("edited", script_event_edited);
    component.register_event_call("submitted", script_event_submitted);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn script_fn_edited(component: &mut brul::Component, mut ev: brul::Dyn) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn script_fn_submitted(component: &mut brul::Component) -> Result<(), brul::Error> {
    Ok(())
}
#[allow(unused_variables)]
fn script_event_edited(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    script_fn_edited(component, (arguments[0]).clone())?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn script_event_submitted(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    script_fn_submitted(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum ScriptMessage {
    Edited(brul::Dyn),
    Submitted,
}
impl From<ScriptMessage> for brul::Message {
    fn from(message: ScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<ScriptMessage>() {
        Some(ScriptMessage::Edited(argument0)) => { script_fn_edited(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        Some(ScriptMessage::Submitted) => { script_fn_submitted(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "edited" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            script_fn_edited(component, arguments.remove(0))?;
            Ok(brul::Dyn::null())
        }
        "submitted" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            script_fn_submitted(component, )?;
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

    pub fn edited(&self, ev: brul::Dyn) {
        self.0.send(ScriptMessage::Edited(ev));
    }

    pub fn submitted(&self) {
        self.0.send(ScriptMessage::Submitted);
    }
}

pub fn names() -> &'static [&'static str] {
    &["draft", "ev"]
}

include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        brul::gui::Widget::view(brul::gui::Direction::Row, vec![
            {
                let (built, _rows) = demo_each1_rebuild(theme, build, component, &[])?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
                brul::register_structure(build, component, widget0.id(), true, demo_each1_value, demo_each1_rebuild);
                widget0
            },
            {
                let widget1 = brul::gui::Widget::text("");
                bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok((&brul::Dyn::num(component.state::<ScriptState>().selected.get())).try_into()?) })));
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

fn demo_each1_value(component: &mut brul::Component) -> Result<brul::Items, brul::Error> {
    Ok(brul::items(&(brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?))))
}

fn demo_each1_key(component: &mut brul::Component) -> Result<brul::Dyn, brul::Error> {
    Ok((brul::Dyn::from(component.read_id(brul::VariableId::from_index(1))?)).field("name")?)
}


#[derive(Clone)]
#[allow(dead_code)]
struct demo_each1_scope {
    item: brul::Dyn,
    index: f64,
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_each1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let mut children = Vec::new();
    let collection = demo_each1_value(&mut component.borrow_mut())
        .map_err(brul::ViewError::from)?;
    let items = collection;
    let mut matcher = brul::RowMatcher::new(old_rows);
    for (position, item) in items.into_iter().enumerate() {
        let previous_scope = component.borrow().scope().clone();
        component.borrow_mut().set_scope(brul::Scope::each(demo_each1_scope {
            item: item.clone(),
            index: position as f64,
        }));
        let key = demo_each1_key(&mut component.borrow_mut()).map(|key| brul::RowKey::Explicit(key.into())).unwrap_or(brul::RowKey::Index(position));
        let start = children.len();
        if let Some(other) = matcher.take(&key, &item, position) {
            rows.push(brul::RowInfo { key, item, start, count: 0, reuse: Some(other) });
            component.borrow_mut().set_scope(previous_scope);
            continue;
        }
        children.extend([
            {
                let widget0 = brul::gui::Widget::text("");
                bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { let data = component.scope().get::<demo_each1_scope>().expect("each scope");
let item = data.item.clone();
let index = data.index.clone();
Ok((&(item.clone()).field("name")?).try_into()?) })));
                widget0
            },
        ]);
        let count = children.len() - start;
        component.borrow_mut().set_scope(previous_scope);
        rows.push(brul::RowInfo { key, item, start, count, reuse: None });
    }
    let widget = brul::gui::Widget::structure(children);
    Ok((brul::Built { widget, bindings, handlers, props }, rows))
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
    pub selected: brul::Signal<f64>,
}

#[allow(dead_code)]
fn script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let selected: f64 = component.seed("selected", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(ScriptState { selected: brul::Signal::new(selected) });
    Ok(())
}

pub fn script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: script_init, call: script_typed_call, dispatch: script_dispatch })
}
#[allow(unused_variables)]
fn script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("bump", script_event_bump);
    component.declare("selected", (brul::Dyn::num(0.0_f64)).into(), false)?;
    script_state_init(component)?;
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn script_fn_bump(component: &mut brul::Component, mut step: f64) -> Result<(), brul::Error> {
    { let value = brul::Dyn::num((component.state::<ScriptState>().selected.get()) + (step));
    component.state::<ScriptState>().selected.set((&value).try_into()?);

value };
    Ok(())
}
#[allow(unused_variables)]
fn script_event_bump(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    script_fn_bump(component, (&arguments[0]).try_into()?)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum ScriptMessage {
    Bump(f64),
}
impl From<ScriptMessage> for brul::Message {
    fn from(message: ScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<ScriptMessage>() {
        Some(ScriptMessage::Bump(argument0)) => { script_fn_bump(component, (argument0).clone())?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "bump" => {
            if arguments.len() != 1 { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 1, found: arguments.len() }); }
            script_fn_bump(component, (&arguments.remove(0)).try_into()?)?;
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

    pub fn bump(&self, step: f64) {
        self.0.send(ScriptMessage::Bump(step));
    }
}

pub fn names() -> &'static [&'static str] {
    &["browsers", "browser", "index", "selected", "step"]
}

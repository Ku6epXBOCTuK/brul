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


#[derive(Clone)]
#[allow(dead_code)]
struct demo_each1_scope {
    item: brul::Dyn,
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
    if items.is_empty() {
        children.extend([
            brul::gui::Widget::text("empty"),
        ]);
        rows.push(brul::RowInfo {
            key: brul::RowKey::Index(0),
            item: brul::Dyn::array(vec![]).into(),
            start: 0,
            count: children.len(),
            reuse: None,
        });
    }
    for (position, item) in items.into_iter().enumerate() {
        let previous_scope = component.borrow().scope().clone();
        component.borrow_mut().set_scope(brul::Scope::each(demo_each1_scope {
            item: item.clone(),
        }));
        let key = brul::RowKey::Index(position);
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

pub fn script() -> Option<brul::Script> {
    None
}

pub fn names() -> &'static [&'static str] {
    &["items", "item"]
}

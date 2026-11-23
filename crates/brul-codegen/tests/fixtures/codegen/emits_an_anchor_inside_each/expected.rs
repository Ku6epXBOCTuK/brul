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
        demo_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
            {
                let (built, _rows) = demo_each1_rebuild(theme, build, component, &[])?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
                brul::register_structure(build, component, widget0.id(), true, demo_each1_value, demo_each1_rebuild);
                widget0
            },
        ]), &[0], theme)
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 3]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
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

pub fn demo_script() -> Option<brul::Script> {
    None
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
            demo_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                {
                    let widget0 = brul::gui::Widget::text("");
                    bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { let data = component.scope().get::<demo_each1_scope>().expect("each scope");
let item = data.item.clone();
let index = data.index.clone();
Ok((&(item.clone()).field("name")?).try_into()?) })));
                    widget0
                },
            ]), &[1], theme).with_id(brul::gui::WidgetId::new(2147483648)),
            {
                let widget1 = brul::gui::Widget::layer(vec![
                    demo_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                        brul::gui::Widget::text("tip"),
                    ]), &[2], theme).anchored(brul::gui::Anchor::new(brul::gui::WidgetId::new(2147483648))),
                ]);
                bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Visible(|component: &mut brul::Component| -> Result<bool, brul::Error> { let data = component.scope().get::<demo_each1_scope>().expect("each scope");
let item = data.item.clone();
let index = data.index.clone();
Ok(true) })));
                widget1
            },
        ]);
        let count = children.len() - start;
        component.borrow_mut().set_scope(previous_scope);
        rows.push(brul::RowInfo { key, item, start, count, reuse: None });
    }
    let widget = brul::gui::Widget::structure(children);
    Ok((brul::Built { widget, bindings, handlers, props }, rows))
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
    &["items", "item", "position"]
}

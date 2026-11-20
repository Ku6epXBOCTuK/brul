include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        styled(brul::gui::Widget::column(vec![]), &[0], theme)
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
    Light,
}

impl ThemeId {
    pub const ALL: &'static [ThemeId] = &[
        ThemeId::Light,
    ];
}

impl brul::Theme for ThemeId {
    fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
        }
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
static CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 59, g: 130, b: 246, a: 128 })), border: Some(brul::gui::Color { r: 114, g: 154, b: 233, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("light") => &*CLASSES_LIGHT,
        Some(_) => &*CLASSES_LIGHT,
    }
}

#[allow(dead_code)]
fn styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "a".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ColorExpr(s::ColorExpr::Alpha { color: s::ColorValue::Theme("accent".into()), amount: 50.0 }) },
                    s::Property { kind: s::Kind::BorderColor, value: s::Value::ColorExpr(s::ColorExpr::Mix { from: s::ColorValue::Color(s::Color::rgba(59, 130, 246, 255)), to: s::ColorValue::Color(s::Color::rgba(255, 255, 255, 255)), amount: 20.0 }) },
                ],
                animations: vec![],
            },
        ],
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
    None
}

pub fn names() -> &'static [&'static str] {
    &[]
}

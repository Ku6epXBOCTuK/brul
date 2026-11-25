include!(concat!(env!("OUT_DIR"), "/allocations.rs"));
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    Dark,
    Light,
}

impl ThemeId {
    pub const ALL: &'static [ThemeId] = &[
        ThemeId::Dark,
        ThemeId::Light,
    ];
}

impl brul::Theme for ThemeId {
    fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
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

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn styled_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("styled", "attach:editor"),
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
        state: brul::StateId::source("styled", "attach:keyboard"),
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
        state: brul::StateId::source("styled", "attach:pointer"),
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
        state: brul::StateId::source("styled", "attach:scroll"),
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
        styled_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![]), &[0], theme)
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static STYLED_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(10)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 108, g: 140, b: 255, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 0, b: 0, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static STYLED_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(10)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 17, g: 34, b: 51, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 0, b: 0, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn styled_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*STYLED_CLASSES_DARK,
        Some("light") => &*STYLED_CLASSES_LIGHT,
        Some(_) => &*STYLED_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn styled_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(108, 140, 255, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(17, 34, 51, 255)) },
            ] },
        ],
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
            s::ClassRule {
                class: "box".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Px(10.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("accent".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "box".into(),
                state: Some(s::State::Checked),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::Color(s::Color::rgba(255, 0, 0, 255)) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn styled_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = styled_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn styled_script() -> Option<brul::Script> {
    None
}

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn styled_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "styled".to_owned(),
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
        brul::WindowDef { name: "Styled", config: styled_window(), view: styled_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    styled_script()
}

pub fn names() -> &'static [&'static str] {
    &[]
}

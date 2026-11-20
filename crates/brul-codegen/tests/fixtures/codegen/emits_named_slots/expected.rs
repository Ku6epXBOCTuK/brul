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
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: badge_view,
                    script: badge_script(),
                    name: "Badge",
                    names: names(),
                    props: &[
                    ],
                    defaults: &[
                    ],
                    events: &[
                    ],
                    slots: &[
                        brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot1 },
                        brul::SlotArgument { name: "header", view: demo_slot2 },
                    ],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
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

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn badge_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        brul::gui::Widget::view(brul::gui::Direction::Row, {
            let mut children = Vec::new();
            {
                let built = brul::slot(theme, build, "header")?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                children.extend(built.widget.into_children());
            }            {
                let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                children.extend(built.widget.into_children());
            }            children
        })
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static BADGE_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn badge_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*BADGE_CLASSES_NONE
}

#[allow(dead_code)]
fn badge_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![],
    });
    &STYLES
}

fn badge_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = badge_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn badge_script() -> Option<brul::Script> {
    None
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot1(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut children = Vec::new();
    children.push(brul::gui::Widget::text("body"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot2(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut children = Vec::new();
    children.push(brul::gui::Widget::text("head"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
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

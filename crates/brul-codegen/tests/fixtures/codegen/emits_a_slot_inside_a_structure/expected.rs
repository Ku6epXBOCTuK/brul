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
                        brul::PropArgument { name: "shown", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                    ],
                    events: &[
                    ],
                    slots: &[
                        brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot1 },
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
    None
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
            children.push({
                let (built, _rows) = badge_if1_rebuild(theme, build, component, &[])?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
                brul::register_structure(build, component, widget0.id(), false, badge_if1_value0, badge_if1_rebuild);
                widget0
            });
            children
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
    children.push(brul::gui::Widget::text("x"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

fn badge_if1_value0(component: &mut brul::Component) -> Result<bool, brul::Error> {
    Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).map_err(|error| error.at(brul::ast::Position { line: 5, column: 14 }))?; value })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn badge_if1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let children = {
        let condition = badge_if1_value0(&mut component.borrow_mut())
            .map_err(brul::ViewError::from)?;
        if condition {
            {
                let mut children = Vec::new();
                {
                    let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
                    bindings.extend(built.bindings);
                    handlers.extend(built.handlers);
                    props.extend(built.props);
                    children.extend(built.widget.into_children());
                }                children
            }
        } else {
            vec![]
        }
    };
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
    demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["shown"]
}

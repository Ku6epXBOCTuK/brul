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
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("demo", "attach:editor"),
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
        state: brul::StateId::source("demo", "attach:keyboard"),
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
        state: brul::StateId::source("demo", "attach:pointer"),
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
        state: brul::StateId::source("demo", "attach:scroll"),
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
        brul::gui::Widget::view(brul::gui::Direction::Row, vec![
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: row_view,
                    script: row_script(),
                    name: "Row",
                    names: names(),
                    props: &[
                    ],
                    defaults: &[
                    ],
                    events: &[
                    ],
                    slots: &[
                        brul::SlotArgument { name: "header", view: demo_slot1 },
                    ],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget.debug_named("Row");
                widget0
            },
        ]).debug_named("Container").debug_source("input.brul")
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

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn badge_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        let mut children = Vec::new();
        {
            let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            children.extend(built.widget.into_children());
        }
        brul::gui::Widget::structure(children)
.debug_source("badge.brul")
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
pub fn row_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        let mut children = Vec::new();
        children.push(row_styled(brul::gui::Widget::text("one"), &[0], theme).debug_named("Text"));
        children.push({
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
                    brul::SlotArgument { name: brul::DEFAULT_SLOT, view: row_slot1 },
                ],
            })?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget0 = built.widget.debug_named("Badge");
            widget0
        });
        children.push({
            let (built, _rows) = row_if1_rebuild(theme, build, component, &[])?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget1 = built.widget;
            brul::register_structure(build, component, widget1.id(), false, row_if1_value0, row_if1_rebuild);
            widget1
        });
        children.push({
            let (built, _rows) = row_each1_rebuild(theme, build, component, &[])?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget2 = built.widget;
            brul::register_structure(build, component, widget2.id(), true, row_each1_value, row_each1_rebuild);
            widget2
        });
        children.push(row_styled(brul::gui::Widget::text("anchored"), &[5], theme).debug_named("Text").anchored(brul::gui::Anchor::new(brul::gui::WidgetId::new(2147483648))));
        children.push(row_styled(brul::gui::Widget::text("three"), &[6], theme).debug_named("Text"));
        {
            let built = brul::slot(theme, build, "header")?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            children.extend(built.widget.into_children());
        }        children.push(row_styled(brul::gui::Widget::text("four"), &[7], theme).debug_named("Text"));
        children.push({
            let (built, _rows) = row_if2_rebuild(theme, build, component, &[])?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget3 = built.widget;
            brul::register_structure(build, component, widget3.id(), false, row_if2_value0, row_if2_rebuild);
            widget3
        });
        children.push({
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
                    brul::SlotArgument { name: brul::DEFAULT_SLOT, view: row_slot2 },
                ],
            })?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget4 = built.widget.debug_named("Badge");
            widget4
        });
        children.push(row_styled(brul::gui::Widget::text("six"), &[9], theme).debug_named("Text"));

        brul::gui::Widget::structure(children)
.debug_source("mixed.brul")
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static ROW_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 10]> = std::sync::LazyLock::new(|| [
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
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { cursor: Some(brul::gui::Cursor::Text), ..brul::gui::WidgetStylePatch::DEFAULT }),
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
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn row_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*ROW_CLASSES_NONE
}

#[allow(dead_code)]
fn row_styles() -> &'static brul::style::Stylesheet {
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

fn row_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = row_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn row_script() -> Option<brul::Script> {
    None
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot1(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let children = vec![
        brul::gui::Widget::text("head").debug_named("Text"),
    ];
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn row_slot1(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let children = vec![
        brul::gui::Widget::text("component").debug_named("Text"),
    ];
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

fn row_if1_value0(component: &mut brul::Component) -> Result<bool, brul::Error> {
    Ok((&brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).try_into()?)
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn row_if1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let children = {
        let condition = row_if1_value0(&mut component.borrow_mut())
            .map_err(brul::ViewError::from)?;
        if condition {
            vec![
                row_styled(brul::gui::Widget::text("two"), &[1], theme).debug_named("Text").with_id(brul::gui::WidgetId::new(2147483648)),
            ]
        } else {
            vec![
                row_styled(brul::gui::Widget::text("none"), &[2], theme).debug_named("Text"),
            ]
        }
    };
    let widget = brul::gui::Widget::structure(children);
    Ok((brul::Built { widget, bindings, handlers, props }, rows))
}

fn row_each1_value(component: &mut brul::Component) -> Result<brul::Items, brul::Error> {
    Ok(brul::items(&(brul::Dyn::from(component.read_id(brul::VariableId::from_index(1))?))))
}


#[derive(Clone)]
#[allow(dead_code)]
struct row_each1_scope {
    item: brul::Dyn,
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn row_each1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let mut children = Vec::new();
    let collection = row_each1_value(&mut component.borrow_mut())
        .map_err(brul::ViewError::from)?;
    let items = collection;
    let mut matcher = brul::RowMatcher::new(old_rows);
    for (position, item) in items.into_iter().enumerate() {
        let previous_scope = component.borrow().scope().clone();
        component.borrow_mut().set_scope(brul::Scope::each(row_each1_scope {
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
                let widget0 = row_styled(brul::gui::Widget::input(""), &[3, 4], theme).debug_named("TextInput");
                bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Input(|component: &mut brul::Component| -> Result<String, brul::Error> { let data = component.scope().get::<row_each1_scope>().expect("each scope");
let item = data.item.clone();
Ok((&item.clone()).try_into()?) })));
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

fn row_if2_value0(component: &mut brul::Component) -> Result<bool, brul::Error> {
    Ok((&brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).try_into()?)
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn row_if2_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let children = {
        let condition = row_if2_value0(&mut component.borrow_mut())
            .map_err(brul::ViewError::from)?;
        if condition {
            vec![
                row_styled(brul::gui::Widget::text("five"), &[8], theme).debug_named("Text"),
            ]
        } else {
            vec![]
        }
    };
    let widget = brul::gui::Widget::structure(children);
    Ok((brul::Built { widget, bindings, handlers, props }, rows))
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn row_slot2(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let children = vec![
        brul::gui::Widget::text("second").debug_named("Text"),
    ];
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
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
    &["shown", "items", "item"]
}

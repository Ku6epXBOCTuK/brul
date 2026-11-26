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
pub fn canvas_demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let _ = build.capability(brul::Capability::hooks(brul::Hooks {
        target: brul::EventTarget::Window,
        state: brul::StateId::source("canvas_demo", "attach:editor"),
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
        state: brul::StateId::source("canvas_demo", "attach:keyboard"),
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
        state: brul::StateId::source("canvas_demo", "attach:pointer"),
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
        state: brul::StateId::source("canvas_demo", "attach:scroll"),
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
        {
            let built = build.capability(brul::Capability::view(brul::StateId::source("canvas_demo", "Canvas/view"), std::rc::Rc::clone(component), brul::Instance {
                theme,
                view: ::brul::canvas::view,
                script: None,
                name: "Canvas",
                names: names(),
                props: &[
                ],
                defaults: &[
                    brul::PropArgument { name: "animated", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                ],
                events: &[
                ],
                slots: &[],
            }))?.expect("view capability");
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget0 = built.widget.debug_named("Canvas");
            let _ = build.capability(brul::Capability::hooks(brul::Hooks {
                target: brul::EventTarget::Widget(widget0.id()),
                state: brul::StateId::source("canvas_demo", "Canvas/attach:waveform"),
                component: std::rc::Rc::clone(component),
                params: vec![],
                apply: crate::__brul_attach_waveform_apply,
                event: crate::__brul_attach_waveform_event,
                raw_event: crate::__brul_attach_waveform_raw_event,
                cleanup: crate::__brul_attach_waveform_cleanup,
                events: brul::EventSet::from([]),
                raw_events: brul::RawEventSet::from([]),
            }));
            widget0
        }.debug_source("input.brul")
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static CANVAS_DEMO_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn canvas_demo_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*CANVAS_DEMO_CLASSES_NONE
}

#[allow(dead_code)]
fn canvas_demo_styles() -> &'static brul::style::Stylesheet {
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

fn canvas_demo_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = canvas_demo_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn canvas_demo_script() -> Option<brul::Script> {
    None
}

pub fn __brul_effects() -> brul::effect::Effects {
    brul::effect::Effects::new(&[
    ])
}

pub fn canvas_demo_window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "canvas".to_owned(),
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
        brul::WindowDef { name: "CanvasDemo", config: canvas_demo_window(), view: canvas_demo_view },
    ]
}

pub fn script() -> Option<brul::Script> {
    canvas_demo_script()
}

pub fn names() -> &'static [&'static str] {
    &["animated"]
}

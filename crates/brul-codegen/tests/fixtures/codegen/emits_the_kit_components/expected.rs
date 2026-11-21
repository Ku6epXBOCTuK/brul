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
                    view: button_view,
                    script: button_script(),
                    name: "Button",
                    names: names(),
                    props: &[
                        brul::PropArgument { name: "label", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("Open".to_owned()) }) },
                        brul::PropArgument { name: "variant", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("primary".to_owned()) }) },
                    ],
                    defaults: &[
                        brul::PropArgument { name: "disabled", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                    ],
                    events: &[
                    ],
                    slots: &[],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget0 = built.widget;
                widget0
            },
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: card_view,
                    script: card_script(),
                    name: "Card",
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
                        brul::SlotArgument { name: "footer", view: demo_slot3 },
                    ],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget1 = built.widget;
                widget1
            },
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: tooltip_view,
                    script: tooltip_script(),
                    name: "Tooltip",
                    names: names(),
                    props: &[
                        brul::PropArgument { name: "text", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("tip".to_owned()) }) },
                    ],
                    defaults: &[
                    ],
                    events: &[
                    ],
                    slots: &[
                        brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot4 },
                    ],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget2 = built.widget;
                widget2
            },
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: checkbox_view,
                    script: checkbox_script(),
                    name: "Checkbox",
                    names: names(),
                    props: &[
                        brul::PropArgument { name: "label", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("Remember".to_owned()) }) },
                    ],
                    defaults: &[
                        brul::PropArgument { name: "checked", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                        brul::PropArgument { name: "disabled", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                    ],
                    events: &[
                    ],
                    slots: &[],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget3 = built.widget;
                widget3
            },
            {
                let built = brul::instance(build, component, brul::Instance {
                    theme,
                    view: dialog_view,
                    script: dialog_script(),
                    name: "Dialog",
                    names: names(),
                    props: &[
                        brul::PropArgument { name: "open", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(component.state::<DemoScriptState>().shown.get()) }) },
                        brul::PropArgument { name: "title", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("Edit".to_owned()) }) },
                    ],
                    defaults: &[
                        brul::PropArgument { name: "confirm", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("Сохранить".to_owned()) }) },
                        brul::PropArgument { name: "cancel", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("Отмена".to_owned()) }) },
                    ],
                    events: &[
                    ],
                    slots: &[
                        brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot5 },
                    ],
                })?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                let widget4 = built.widget;
                widget4
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
static DEMO_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static DEMO_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn demo_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*DEMO_CLASSES_DARK,
        Some("light") => &*DEMO_CLASSES_LIGHT,
        Some(_) => &*DEMO_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn demo_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
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

#[allow(dead_code)]
pub struct DemoScriptState {
    pub shown: brul::Signal<bool>,
    pub draft: brul::Signal<String>,
}

#[allow(dead_code)]
fn demo_script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let shown: bool = component.seed("shown", brul::ast::Position { line: 0, column: 0 })?;
    let draft: String = component.seed("draft", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(DemoScriptState { shown: brul::Signal::new(shown), draft: brul::Signal::new(draft) });
    Ok(())
}

pub fn demo_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: demo_script_init, call: demo_script_typed_call, dispatch: demo_script_dispatch })
}
#[allow(unused_variables)]
fn demo_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.declare("shown", (brul::Dyn::flag(false)).into(), false)?;
    component.declare("draft", (brul::Dyn::text("".to_owned())).into(), false)?;
    demo_script_state_init(component)?;
    Ok(())
}

#[allow(dead_code)]
pub enum DemoScriptMessage {
}
impl From<DemoScriptMessage> for brul::Message {
    fn from(message: DemoScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn demo_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<DemoScriptMessage>() {
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn demo_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct DemoScript(brul::Handle);

#[allow(dead_code)]
impl DemoScript {
    pub fn new(handle: brul::Handle) -> Self {
        Self(handle)
    }
}

#[allow(dead_code)]
pub type Script = DemoScript;

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn button_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = button_styled(brul::gui::Widget::view(brul::gui::Direction::Row, {
                let mut children = Vec::new();
                {
                    let built = brul::slot(theme, build, "icon")?;
                    bindings.extend(built.bindings);
                    handlers.extend(built.handlers);
                    props.extend(built.props);
                    children.extend(built.widget.into_children());
                }                children.push({
                    let widget1 = button_styled(brul::gui::Widget::text(""), &[1], theme);
                    bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(2))?)).map_err(|error| error.at(brul::ast::Position { line: 16, column: 35 }))?; value }) })));
                    widget1
                });
                children
            }), &[0], theme).focusable(true);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Disabled(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(4))?)).map_err(|error| error.at(brul::ast::Position { line: 14, column: 71 }))?; value }) })));
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Class { table: button_class_table(theme), expression: |component: &mut brul::Component| -> Result<brul::Dyn, brul::Error> { let value = brul::Dyn::text({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(3))?)).map_err(|error| error.at(brul::ast::Position { line: 14, column: 35 }))?; value }); Ok(value) } }));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Click, button_script_event_press));
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
static BUTTON_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 2]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { height: Some(brul::gui::Dimension::Px(32)), padding: Some(brul::gui::Padding { left: 16, top: 0, right: 16, bottom: 0 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), gap: Some(6), justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 30, g: 41, b: 59, a: 255 })), border_left: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_top: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_right: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_bottom: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), shadow: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_radius: Some(10), shadow_x: Some(0), shadow_y: Some(0), shadow_blur: Some(4), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Hovered, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 39, g: 53, b: 73, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Focused, brul::gui::WidgetStylePatch { border_left: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_top: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_right: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_bottom: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), shadow: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Disabled, brul::gui::WidgetStylePatch { opacity: Some(0.45), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { background: Some(std::time::Duration::from_millis(200)), ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 248, g: 250, b: 252, a: 255 }), vertical_align: Some(brul::gui::VerticalAlign::Center), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { weight: Some(600), size: Some(13.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static BUTTON_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 2]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { height: Some(brul::gui::Dimension::Px(32)), padding: Some(brul::gui::Padding { left: 16, top: 0, right: 16, bottom: 0 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), gap: Some(6), justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 241, g: 245, b: 249, a: 255 })), border_left: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_top: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_right: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_bottom: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), shadow: Some(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 }), border_radius: Some(10), shadow_x: Some(0), shadow_y: Some(0), shadow_blur: Some(4), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Hovered, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 226, g: 232, b: 240, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Focused, brul::gui::WidgetStylePatch { border_left: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_top: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_right: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_bottom: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), shadow: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Disabled, brul::gui::WidgetStylePatch { opacity: Some(0.45), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { background: Some(std::time::Duration::from_millis(200)), ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 15, g: 23, b: 42, a: 255 }), vertical_align: Some(brul::gui::VerticalAlign::Center), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { weight: Some(600), size: Some(13.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn button_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*BUTTON_CLASSES_DARK,
        Some("light") => &*BUTTON_CLASSES_LIGHT,
        Some(_) => &*BUTTON_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn button_class_dark(name: &str) -> Option<brul::gui::Class> {
    brul::gui::style::class(button_styles(), name, Some("dark")).ok()
}

#[allow(dead_code)]
fn button_class_light(name: &str) -> Option<brul::gui::Class> {
    brul::gui::style::class(button_styles(), name, Some("light")).ok()
}

fn button_class_table(theme: Option<&str>) -> brul::ClassTable {
    match theme { None | Some("dark") => button_class_dark, Some("light") => button_class_light, Some(_) => button_class_dark }
}

#[allow(dead_code)]
fn button_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "button".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Height, value: s::Value::Px(32.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Left), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Right), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(6.0) },
                    s::Property { kind: s::Kind::AlignItems, value: s::Value::Keyword("center".into()) },
                    s::Property { kind: s::Kind::JustifyContent, value: s::Value::Keyword("center".into()) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Top), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Right), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Bottom), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Left), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(10.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                    s::Property { kind: s::Kind::ShadowOffsetX, value: s::Value::Px(0.0) },
                    s::Property { kind: s::Kind::ShadowOffsetY, value: s::Value::Px(0.0) },
                    s::Property { kind: s::Kind::ShadowBlur, value: s::Value::Px(4.0) },
                    s::Property { kind: s::Kind::ShadowColor, value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                ],
                animations: vec![
                    s::Animation { property: s::Kind::Background, duration: 200.0 },
                ],
            },
            s::ClassRule {
                class: "button".into(),
                state: Some(s::State::Hover),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface-hover".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "button".into(),
                state: Some(s::State::Focus),
                properties: vec![
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::ShadowColor, value: s::Value::ThemeRef("accent".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "button".into(),
                state: Some(s::State::Disabled),
                properties: vec![
                    s::Property { kind: s::Kind::Opacity, value: s::Value::Number(0.45) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "secondary".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("border".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "secondary".into(),
                state: Some(s::State::Hover),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface-hover".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "primary".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("accent".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "primary".into(),
                state: Some(s::State::Hover),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("accent-hover".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "ghost".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "ghost".into(),
                state: Some(s::State::Hover),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "button-label".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Color, value: s::Value::ThemeRef("text".into()) },
                    s::Property { kind: s::Kind::FontSize, value: s::Value::Px(13.0) },
                    s::Property { kind: s::Kind::FontWeight, value: s::Value::Number(600.0) },
                    s::Property { kind: s::Kind::VerticalAlign, value: s::Value::Keyword("center".into()) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn button_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = button_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn button_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: button_script_init, call: button_script_typed_call, dispatch: button_script_dispatch })
}
#[allow(unused_variables)]
fn button_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("press", button_script_event_press);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn button_script_fn_press(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let arguments = vec![]; brul::Dyn::call(component, "clicked", arguments)? };
    Ok(())
}
#[allow(unused_variables)]
fn button_script_event_press(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    button_script_fn_press(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum ButtonScriptMessage {
    Press,
}
impl From<ButtonScriptMessage> for brul::Message {
    fn from(message: ButtonScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn button_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<ButtonScriptMessage>() {
        Some(ButtonScriptMessage::Press) => { button_script_fn_press(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn button_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "press" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            button_script_fn_press(component, )?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn card_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        card_styled(brul::gui::Widget::view(brul::gui::Direction::Row, {
            let mut children = Vec::new();
            {
                let built = brul::slot(theme, build, "header")?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                children.extend(built.widget.into_children());
            }            children.push(card_styled(brul::gui::Widget::view(brul::gui::Direction::Row, {
                let mut children = Vec::new();
                {
                    let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
                    bindings.extend(built.bindings);
                    handlers.extend(built.handlers);
                    props.extend(built.props);
                    children.extend(built.widget.into_children());
                }                children
            }), &[1], theme));
            {
                let built = brul::slot(theme, build, "footer")?;
                bindings.extend(built.bindings);
                handlers.extend(built.handlers);
                props.extend(built.props);
                children.extend(built.widget.into_children());
            }            children
        }), &[0], theme)
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static CARD_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 2]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 30, g: 41, b: 59, a: 255 })), border_left: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_top: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_right: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_bottom: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), clip: Some(true), border_radius: Some(14), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), padding: Some(brul::gui::Padding { left: 16, top: 16, right: 16, bottom: 16 }), gap: Some(8), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static CARD_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 2]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 241, g: 245, b: 249, a: 255 })), border_left: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_top: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_right: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_bottom: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), clip: Some(true), border_radius: Some(14), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), padding: Some(brul::gui::Padding { left: 16, top: 16, right: 16, bottom: 16 }), gap: Some(8), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn card_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*CARD_CLASSES_DARK,
        Some("light") => &*CARD_CLASSES_LIGHT,
        Some(_) => &*CARD_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn card_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "card".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Top), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Right), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Bottom), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Left), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(14.0) },
                    s::Property { kind: s::Kind::Clip, value: s::Value::Bool(true) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "card-body".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Top), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Right), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Bottom), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Left), value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(8.0) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn card_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = card_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn card_script() -> Option<brul::Script> {
    None
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn tooltip_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = tooltip_styled(brul::gui::Widget::view(brul::gui::Direction::Row, {
                let mut children = Vec::new();
                {
                    let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
                    bindings.extend(built.bindings);
                    handlers.extend(built.handlers);
                    props.extend(built.props);
                    children.extend(built.widget.into_children());
                }                children.push({
                    let widget1 = tooltip_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                        {
                            let widget2 = tooltip_styled(brul::gui::Widget::text(""), &[2], theme);
                            bindings.push(brul::Binding::new(component, widget2.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(5))?)).map_err(|error| error.at(brul::ast::Position { line: 20, column: 38 }))?; value }) })));
                            widget2
                        },
                    ]), &[1], theme);
                    bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Visible(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(component.state::<TooltipScriptState>().shown.get()) })));
                    widget1
                });
                children
            }), &[0], theme);
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Enter, tooltip_script_event_show));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Leave, tooltip_script_event_hide));
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
static TOOLTIP_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 3]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(6), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { padding: Some(brul::gui::Padding { left: 12, top: 8, right: 12, bottom: 8 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 30, g: 41, b: 59, a: 255 })), border_left: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_top: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_right: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_bottom: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_radius: Some(8), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { size: Some(12.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static TOOLTIP_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 3]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(6), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { padding: Some(brul::gui::Padding { left: 12, top: 8, right: 12, bottom: 8 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 241, g: 245, b: 249, a: 255 })), border_left: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_top: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_right: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_bottom: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_radius: Some(8), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 71, g: 85, b: 105, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { size: Some(12.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn tooltip_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*TOOLTIP_CLASSES_DARK,
        Some("light") => &*TOOLTIP_CLASSES_LIGHT,
        Some(_) => &*TOOLTIP_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn tooltip_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "tooltip".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(6.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "bubble".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Top), value: s::Value::Px(8.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Right), value: s::Value::Px(12.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Bottom), value: s::Value::Px(8.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Left), value: s::Value::Px(12.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Top), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Right), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Bottom), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Left), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(8.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "bubble-text".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Color, value: s::Value::ThemeRef("muted".into()) },
                    s::Property { kind: s::Kind::FontSize, value: s::Value::Px(12.0) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn tooltip_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = tooltip_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

#[allow(dead_code)]
pub struct TooltipScriptState {
    pub shown: brul::Signal<bool>,
}

#[allow(dead_code)]
fn tooltip_script_state_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    let shown: bool = component.seed("shown", brul::ast::Position { line: 0, column: 0 })?;
    component.set_state(TooltipScriptState { shown: brul::Signal::new(shown) });
    Ok(())
}

pub fn tooltip_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: tooltip_script_init, call: tooltip_script_typed_call, dispatch: tooltip_script_dispatch })
}
#[allow(unused_variables)]
fn tooltip_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("show", tooltip_script_event_show);
    component.register_event_call("hide", tooltip_script_event_hide);
    component.declare("shown", (brul::Dyn::flag(false)).into(), false)?;
    tooltip_script_state_init(component)?;
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn tooltip_script_fn_show(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let value = brul::Dyn::flag(true);
    component.state::<TooltipScriptState>().shown.set((&value).try_into()?);

value };
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn tooltip_script_fn_hide(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let value = brul::Dyn::flag(false);
    component.state::<TooltipScriptState>().shown.set((&value).try_into()?);

value };
    Ok(())
}
#[allow(unused_variables)]
fn tooltip_script_event_show(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    tooltip_script_fn_show(component)?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn tooltip_script_event_hide(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    tooltip_script_fn_hide(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum TooltipScriptMessage {
    Show,
    Hide,
}
impl From<TooltipScriptMessage> for brul::Message {
    fn from(message: TooltipScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn tooltip_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<TooltipScriptMessage>() {
        Some(TooltipScriptMessage::Show) => { tooltip_script_fn_show(component)?; Ok(brul::Dyn::null()) }
        Some(TooltipScriptMessage::Hide) => { tooltip_script_fn_hide(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn tooltip_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "show" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            tooltip_script_fn_show(component, )?;
            Ok(brul::Dyn::null())
        }
        "hide" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            tooltip_script_fn_hide(component, )?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn checkbox_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = checkbox_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                {
                    let widget1 = checkbox_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                        {
                            let widget2 = checkbox_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![]), &[2], theme);
                            bindings.push(brul::Binding::new(component, widget2.id(), brul::BindingTarget::Visible(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(6))?)).map_err(|error| error.at(brul::ast::Position { line: 19, column: 43 }))?; value }) })));
                            widget2
                        },
                    ]), &[1], theme);
                    bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Checked(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(6))?)).map_err(|error| error.at(brul::ast::Position { line: 18, column: 38 }))?; value }) })));
                    widget1
                },
                {
                    let (built, _rows) = checkbox_if1_rebuild(theme, build, component, &[])?;
                    bindings.extend(built.bindings);
                    handlers.extend(built.handlers);
                    props.extend(built.props);
                    let widget3 = built.widget;
                    brul::register_structure(build, component, widget3.id(), false, checkbox_if1_value0, checkbox_if1_rebuild);
                    widget3
                },
            ]), &[0], theme);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Disabled(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(4))?)).map_err(|error| error.at(brul::ast::Position { line: 17, column: 40 }))?; value }) })));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Click, checkbox_script_event_toggle));
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
static CHECKBOX_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 4]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), height: Some(brul::gui::Dimension::Px(28)), gap: Some(10), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Disabled, brul::gui::WidgetStylePatch { opacity: Some(0.45), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(18)), height: Some(brul::gui::Dimension::Px(18)), border_width: Some(brul::gui::BorderWidths { left: 2, top: 2, right: 2, bottom: 2 }), justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 })), border_left: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_top: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_right: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_bottom: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_radius: Some(5), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 })), border_left: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_top: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_right: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), border_bottom: Some(brul::gui::Color { r: 59, g: 130, b: 246, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(6)), height: Some(brul::gui::Dimension::Px(6)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 255, b: 255, a: 255 })), border_radius: Some(3), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 248, g: 250, b: 252, a: 255 }), vertical_align: Some(brul::gui::VerticalAlign::Center), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { size: Some(13.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static CHECKBOX_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 4]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), height: Some(brul::gui::Dimension::Px(28)), gap: Some(10), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Disabled, brul::gui::WidgetStylePatch { opacity: Some(0.45), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(18)), height: Some(brul::gui::Dimension::Px(18)), border_width: Some(brul::gui::BorderWidths { left: 2, top: 2, right: 2, bottom: 2 }), justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 0, g: 0, b: 0, a: 0 })), border_left: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_top: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_right: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_bottom: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_radius: Some(5), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 })), border_left: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_top: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_right: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), border_bottom: Some(brul::gui::Color { r: 37, g: 99, b: 235, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(6)), height: Some(brul::gui::Dimension::Px(6)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 255, b: 255, a: 255 })), border_radius: Some(3), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 15, g: 23, b: 42, a: 255 }), vertical_align: Some(brul::gui::VerticalAlign::Center), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { size: Some(13.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn checkbox_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*CHECKBOX_CLASSES_DARK,
        Some("light") => &*CHECKBOX_CLASSES_LIGHT,
        Some(_) => &*CHECKBOX_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn checkbox_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "checkbox".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Height, value: s::Value::Px(28.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(10.0) },
                    s::Property { kind: s::Kind::AlignItems, value: s::Value::Keyword("center".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "checkbox".into(),
                state: Some(s::State::Disabled),
                properties: vec![
                    s::Property { kind: s::Kind::Opacity, value: s::Value::Number(0.45) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "box".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Px(18.0) },
                    s::Property { kind: s::Kind::Height, value: s::Value::Px(18.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Top), value: s::Value::Px(2.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Right), value: s::Value::Px(2.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Bottom), value: s::Value::Px(2.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Left), value: s::Value::Px(2.0) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(5.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::Color(s::Color::rgba(0, 0, 0, 0)) },
                    s::Property { kind: s::Kind::AlignItems, value: s::Value::Keyword("center".into()) },
                    s::Property { kind: s::Kind::JustifyContent, value: s::Value::Keyword("center".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "box".into(),
                state: Some(s::State::Checked),
                properties: vec![
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("accent".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("accent".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "mark".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Px(6.0) },
                    s::Property { kind: s::Kind::Height, value: s::Value::Px(6.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::Color(s::Color::rgba(255, 255, 255, 255)) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(3.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "checkbox-label".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Color, value: s::Value::ThemeRef("text".into()) },
                    s::Property { kind: s::Kind::FontSize, value: s::Value::Px(13.0) },
                    s::Property { kind: s::Kind::VerticalAlign, value: s::Value::Keyword("center".into()) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn checkbox_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = checkbox_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn checkbox_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: checkbox_script_init, call: checkbox_script_typed_call, dispatch: checkbox_script_dispatch })
}
#[allow(unused_variables)]
fn checkbox_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("toggle", checkbox_script_event_toggle);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn checkbox_script_fn_toggle(component: &mut brul::Component) -> Result<(), brul::Error> {
    if !({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(4))?)).map_err(|error| error.at(brul::ast::Position { line: 3, column: 14 }))?; value }) {
    { let value = brul::Dyn::flag(!({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(6))?)).map_err(|error| error.at(brul::ast::Position { line: 4, column: 24 }))?; value }));
component.assign_id(brul::VariableId::from_index(6), (value.clone()).into())?;

value };
    { let arguments = vec![]; brul::Dyn::call(component, "toggled", arguments)? };
    }
    Ok(())
}
#[allow(unused_variables)]
fn checkbox_script_event_toggle(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    checkbox_script_fn_toggle(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum CheckboxScriptMessage {
    Toggle,
}
impl From<CheckboxScriptMessage> for brul::Message {
    fn from(message: CheckboxScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn checkbox_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<CheckboxScriptMessage>() {
        Some(CheckboxScriptMessage::Toggle) => { checkbox_script_fn_toggle(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn checkbox_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "toggle" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            checkbox_script_fn_toggle(component, )?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn dialog_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = dialog_styled(brul::gui::Widget::layer(vec![
                {
                    let widget1 = dialog_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![]), &[1], theme);
                    bindings.push(brul::Binding::new(component, widget1.id(), brul::BindingTarget::Offset { x: Some(|component: &mut brul::Component| -> Result<f64, brul::Error> { Ok(0.0_f64) }), y: Some(|component: &mut brul::Component| -> Result<f64, brul::Error> { Ok(0.0_f64) }) }));
                    handlers.push(brul::Handler::new(component, Some(widget1.id()), brul::HandlerEvent::Click, dialog_script_event_dismiss_dialog));
                    widget1
                },
                dialog_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                    {
                        let widget2 = dialog_styled(brul::gui::Widget::text(""), &[3], theme);
                        bindings.push(brul::Binding::new(component, widget2.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(8))?)).map_err(|error| error.at(brul::ast::Position { line: 30, column: 39 }))?; value }) })));
                        widget2
                    },
                    dialog_styled(brul::gui::Widget::view(brul::gui::Direction::Row, {
                        let mut children = Vec::new();
                        {
                            let built = brul::slot(theme, build, brul::DEFAULT_SLOT)?;
                            bindings.extend(built.bindings);
                            handlers.extend(built.handlers);
                            props.extend(built.props);
                            children.extend(built.widget.into_children());
                        }                        children
                    }), &[4], theme),
                    dialog_styled(brul::gui::Widget::view(brul::gui::Direction::Row, vec![
                        {
                            let built = brul::instance(build, component, brul::Instance {
                                theme,
                                view: button_view,
                                script: button_script(),
                                name: "Button",
                                names: names(),
                                props: &[
                                    brul::PropArgument { name: "label", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(10))?)).map_err(|error| error.at(brul::ast::Position { line: 33, column: 36 }))?; value }) }) },
                                ],
                                defaults: &[
                                    brul::PropArgument { name: "variant", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("secondary".to_owned()) }) },
                                    brul::PropArgument { name: "disabled", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                                ],
                                events: &[
                                    brul::ComponentEvent { name: "clicked", call: dialog_script_event_reject, params: &[], component: std::rc::Rc::clone(component) },
                                ],
                                slots: &[],
                            })?;
                            bindings.extend(built.bindings);
                            handlers.extend(built.handlers);
                            props.extend(built.props);
                            let widget3 = built.widget;
                            widget3
                        },
                        {
                            let built = brul::instance(build, component, brul::Instance {
                                theme,
                                view: button_view,
                                script: button_script(),
                                name: "Button",
                                names: names(),
                                props: &[
                                    brul::PropArgument { name: "label", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(9))?)).map_err(|error| error.at(brul::ast::Position { line: 34, column: 36 }))?; value }) }) },
                                    brul::PropArgument { name: "variant", ty: brul::PropType::String, expression: brul::PropExpression::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok("primary".to_owned()) }) },
                                ],
                                defaults: &[
                                    brul::PropArgument { name: "disabled", ty: brul::PropType::Bool, expression: brul::PropExpression::Flag(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok(false) }) },
                                ],
                                events: &[
                                    brul::ComponentEvent { name: "clicked", call: dialog_script_event_accept, params: &[], component: std::rc::Rc::clone(component) },
                                ],
                                slots: &[],
                            })?;
                            bindings.extend(built.bindings);
                            handlers.extend(built.handlers);
                            props.extend(built.props);
                            let widget4 = built.widget;
                            widget4
                        },
                    ]), &[5], theme),
                ]), &[2], theme),
            ]), &[0], theme).modal(true);
            bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Visible(|component: &mut brul::Component| -> Result<bool, brul::Error> { Ok({ let value = bool::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(7))?)).map_err(|error| error.at(brul::ast::Position { line: 27, column: 39 }))?; value }) })));
            handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Dismiss, dialog_script_event_dismiss_dialog));
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
static DIALOG_CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 6]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), height: Some(brul::gui::Dimension::Percent(100.0)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 0, g: 0, b: 0, a: 115 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(420)), padding: Some(brul::gui::Padding { left: 20, top: 20, right: 20, bottom: 20 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), gap: Some(16), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 30, g: 41, b: 59, a: 255 })), border_left: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_top: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_right: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_bottom: Some(brul::gui::Color { r: 51, g: 65, b: 85, a: 255 }), border_radius: Some(14), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 248, g: 250, b: 252, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { weight: Some(600), size: Some(16.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(12), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(8), justify: Some(brul::gui::Justify::End), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static DIALOG_CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 6]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { justify: Some(brul::gui::Justify::Center), align: Some(brul::gui::Align::Center), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), height: Some(brul::gui::Dimension::Percent(100.0)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 0, g: 0, b: 0, a: 115 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(420)), padding: Some(brul::gui::Padding { left: 20, top: 20, right: 20, bottom: 20 }), border_width: Some(brul::gui::BorderWidths { left: 1, top: 1, right: 1, bottom: 1 }), gap: Some(16), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 241, g: 245, b: 249, a: 255 })), border_left: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_top: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_right: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_bottom: Some(brul::gui::Color { r: 203, g: 213, b: 225, a: 255 }), border_radius: Some(14), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { text: Some(brul::gui::Color { r: 15, g: 23, b: 42, a: 255 }), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { weight: Some(600), size: Some(16.0), ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(12), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Percent(100.0)), gap: Some(8), justify: Some(brul::gui::Justify::End), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn dialog_classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*DIALOG_CLASSES_DARK,
        Some("light") => &*DIALOG_CLASSES_LIGHT,
        Some(_) => &*DIALOG_CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn dialog_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![
            s::Theme { name: "dark".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(30, 41, 59, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(39, 53, 73, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(59, 130, 246, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(248, 250, 252, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(51, 65, 85, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
            ] },
            s::Theme { name: "light".into(), properties: vec![
                s::ThemeEntry { key: "surface".into(), value: s::Value::Color(s::Color::rgba(241, 245, 249, 255)) },
                s::ThemeEntry { key: "surface-hover".into(), value: s::Value::Color(s::Color::rgba(226, 232, 240, 255)) },
                s::ThemeEntry { key: "accent".into(), value: s::Value::Color(s::Color::rgba(37, 99, 235, 255)) },
                s::ThemeEntry { key: "accent-hover".into(), value: s::Value::Color(s::Color::rgba(29, 78, 216, 255)) },
                s::ThemeEntry { key: "text".into(), value: s::Value::Color(s::Color::rgba(15, 23, 42, 255)) },
                s::ThemeEntry { key: "border".into(), value: s::Value::Color(s::Color::rgba(203, 213, 225, 255)) },
                s::ThemeEntry { key: "muted".into(), value: s::Value::Color(s::Color::rgba(71, 85, 105, 255)) },
            ] },
        ],
        rules: vec![
            s::ClassRule {
                class: "dialog-layer".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::AlignItems, value: s::Value::Keyword("center".into()) },
                    s::Property { kind: s::Kind::JustifyContent, value: s::Value::Keyword("center".into()) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "dialog-scrim".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Height, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ColorExpr(s::ColorExpr::Alpha { color: s::ColorValue::Color(s::Color::rgba(0, 0, 0, 255)), amount: 45.0 }) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "dialog-card".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Px(420.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Top), value: s::Value::Px(20.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Right), value: s::Value::Px(20.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Bottom), value: s::Value::Px(20.0) },
                    s::Property { kind: s::Kind::PaddingSide(s::Side::Left), value: s::Value::Px(20.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::Background, value: s::Value::ThemeRef("surface".into()) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Top), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Right), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Bottom), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderWidthSide(s::Side::Left), value: s::Value::Px(1.0) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Top), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Right), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Bottom), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderColorSide(s::Side::Left), value: s::Value::ThemeRef("border".into()) },
                    s::Property { kind: s::Kind::BorderRadius, value: s::Value::Px(14.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "dialog-title".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Color, value: s::Value::ThemeRef("text".into()) },
                    s::Property { kind: s::Kind::FontSize, value: s::Value::Px(16.0) },
                    s::Property { kind: s::Kind::FontWeight, value: s::Value::Number(600.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "dialog-body".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(12.0) },
                ],
                animations: vec![],
            },
            s::ClassRule {
                class: "dialog-actions".into(),
                state: None,
                properties: vec![
                    s::Property { kind: s::Kind::Width, value: s::Value::Percent(100.0) },
                    s::Property { kind: s::Kind::JustifyContent, value: s::Value::Keyword("end".into()) },
                    s::Property { kind: s::Kind::Gap, value: s::Value::Px(8.0) },
                ],
                animations: vec![],
            },
        ],
    });
    &STYLES
}

fn dialog_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = dialog_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn dialog_script() -> Option<brul::Script> {
    Some(brul::Script::Typed { init: dialog_script_init, call: dialog_script_typed_call, dispatch: dialog_script_dispatch })
}
#[allow(unused_variables)]
fn dialog_script_init(component: &mut brul::Component) -> Result<(), brul::Error> {
    component.register_event_call("accept", dialog_script_event_accept);
    component.register_event_call("reject", dialog_script_event_reject);
    component.register_event_call("dismissDialog", dialog_script_event_dismiss_dialog);
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn dialog_script_fn_accept(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let arguments = vec![brul::Dyn::flag(true)]; brul::Dyn::call(component, "closed", arguments)? };
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn dialog_script_fn_reject(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let arguments = vec![brul::Dyn::flag(false)]; brul::Dyn::call(component, "closed", arguments)? };
    Ok(())
}
#[allow(unused_mut, unused_variables)]
fn dialog_script_fn_dismiss_dialog(component: &mut brul::Component) -> Result<(), brul::Error> {
    { let arguments = vec![brul::Dyn::flag(false)]; brul::Dyn::call(component, "closed", arguments)? };
    Ok(())
}
#[allow(unused_variables)]
fn dialog_script_event_accept(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    dialog_script_fn_accept(component)?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn dialog_script_event_reject(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    dialog_script_fn_reject(component)?;
    Ok(brul::Dyn::null())
}
#[allow(unused_variables)]
fn dialog_script_event_dismiss_dialog(component: &mut brul::Component, arguments: &[brul::Dyn]) -> Result<brul::Dyn, brul::Error> {
    dialog_script_fn_dismiss_dialog(component)?;
    Ok(brul::Dyn::null())
}

#[allow(dead_code)]
pub enum DialogScriptMessage {
    Accept,
    Reject,
    DismissDialog,
}
impl From<DialogScriptMessage> for brul::Message {
    fn from(message: DialogScriptMessage) -> Self {
        brul::Message::Invoke(Box::new(message))
    }
}

#[allow(unused_variables)]
fn dialog_script_typed_call(component: &mut brul::Component, message: &dyn std::any::Any) -> Result<brul::Dyn, brul::Error> {
    match message.downcast_ref::<DialogScriptMessage>() {
        Some(DialogScriptMessage::Accept) => { dialog_script_fn_accept(component)?; Ok(brul::Dyn::null()) }
        Some(DialogScriptMessage::Reject) => { dialog_script_fn_reject(component)?; Ok(brul::Dyn::null()) }
        Some(DialogScriptMessage::DismissDialog) => { dialog_script_fn_dismiss_dialog(component)?; Ok(brul::Dyn::null()) }
        _ => Err(brul::Error::Unsupported("script message")),
    }
}
#[allow(unused_mut)]
fn dialog_script_dispatch(component: &mut brul::Component, name: &str, mut arguments: Vec<brul::Dyn>) -> Result<brul::Dyn, brul::Error> {
    match name {
        "accept" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            dialog_script_fn_accept(component, )?;
            Ok(brul::Dyn::null())
        }
        "reject" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            dialog_script_fn_reject(component, )?;
            Ok(brul::Dyn::null())
        }
        "dismissDialog" => {
            if !arguments.is_empty() { return Err(brul::Error::FunctionArity { name: name.to_owned(), expected: 0, found: arguments.len() }); }
            dialog_script_fn_dismiss_dialog(component, )?;
            Ok(brul::Dyn::null())
        }
        _ => Err(brul::Error::UnknownFunction(name.to_owned())),
    }
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
    children.push(brul::gui::Widget::text("header"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot3(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut children = Vec::new();
    children.push(brul::gui::Widget::text("footer"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot4(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut children = Vec::new();
    children.push(brul::gui::Widget::text("trigger"));
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn demo_slot5(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut children = Vec::new();
    children.push({
        let widget0 = demo_styled(brul::gui::Widget::input(""), &[0], theme);
        bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Input(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok(component.state::<DemoScriptState>().draft.get()) })));
        widget0
    });
    let widget = brul::gui::Widget::view(brul::gui::Direction::Column, children);
    Ok(brul::Built { widget, bindings, handlers, props })
}

fn checkbox_if1_value0(component: &mut brul::Component) -> Result<bool, brul::Error> {
    Ok(({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(2))?)).map_err(|error| error.at(brul::ast::Position { line: 21, column: 18 }))?; value }).as_str() != ("".to_owned()).as_str())
}

#[allow(unused_mut, unused_variables, clippy::let_and_return)]
fn checkbox_if1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let mut rows = Vec::new();
    let children = {
        let condition = checkbox_if1_value0(&mut component.borrow_mut())
            .map_err(brul::ViewError::from)?;
        if condition {
            vec![
                {
                    let widget0 = checkbox_styled(brul::gui::Widget::text(""), &[3], theme);
                    bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<String, brul::Error> { Ok({ let value = String::try_from(brul::Dyn::from(component.read_id(brul::VariableId::from_index(2))?)).map_err(|error| error.at(brul::ast::Position { line: 22, column: 41 }))?; value }) })));
                    widget0
                },
            ]
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
    &["shown", "draft", "label", "variant", "disabled", "text", "checked", "open", "title", "confirm", "cancel"]
}

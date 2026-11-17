use brul_codegen::emit;

#[test]
fn emits_static_tree() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Row {
        Text { "a" }
        View { }
    }
}
</markup>
"#;
    let expected = r#"pub fn demo_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    let theme = theme.map(brul::Theme::name);
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        brul::gui::Widget::row(vec![
            brul::gui::Widget::text("a"),
            brul::gui::Widget::column(vec![]),
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

pub fn script() -> Option<&'static brul::script::Program> {
    None
}

pub fn names() -> &'static [&'static str] {
    &[]
}
"#;
    assert_eq!(views(emit::module(source).expect("module")), expected);
}

fn views(module: String) -> String {
    module
        .split_once("\npub fn ")
        .map(|(_, rest)| format!("pub fn {rest}"))
        .expect("a generated view")
}

#[test]
fn emits_the_allocation_counter() {
    let module = emit::module(
        r#"<markup>
window Demo (title: "demo") {
    View { }
}
</markup>
"#,
    )
    .expect("module");

    assert!(module.contains("include!(concat!(env!(\"OUT_DIR\"), \"/allocations.rs\"));"));
    let allocations = emit::allocations();
    assert!(allocations.contains("#[cfg(brul_alloc_count)]\nmod brul_allocations {"));
    assert!(allocations.contains("#[global_allocator]"));
    assert!(allocations.contains("REALLOCATIONS.fetch_add(1, Ordering::Relaxed);"));
    assert!(
        module.contains("brul::perf::measure(brul_allocations::reset, brul_allocations::report);")
    );
}

#[test]
fn emits_themes_and_states() {
    let source = r#"<style>
theme dark {
    accent: #6c8cff;
}
theme light {
    accent: #112233;
}
.box {
    width: 10px;
    background: theme.accent;
}
.box:checked {
    background: #ff0000;
}
</style>

<markup>
window Styled (title: "styled") {
    View (box) { }
}
</markup>
"#;
    let expected = r#"pub fn styled_view(theme: Option<ThemeId>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
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

#[allow(dead_code)]
static CLASSES_DARK: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(10)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 108, g: 140, b: 255, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 0, b: 0, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

#[allow(dead_code)]
static CLASSES_LIGHT: std::sync::LazyLock<[brul::gui::Class; 1]> = std::sync::LazyLock::new(|| [
    brul::gui::Class {
        layout: brul::gui::LayoutPatch { width: Some(brul::gui::Dimension::Px(10)), ..brul::gui::LayoutPatch::DEFAULT },
        style: brul::gui::Layers::new(brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 17, g: 34, b: 51, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT })
            .with(brul::gui::StyleState::Checked, brul::gui::WidgetStylePatch { background: Some(brul::gui::Paint::Color(brul::gui::Color { r: 255, g: 0, b: 0, a: 255 })), ..brul::gui::WidgetStylePatch::DEFAULT }),
        text: brul::gui::Layers::new(brul::gui::TextStylePatch { ..brul::gui::TextStylePatch::DEFAULT }),
        transitions: brul::gui::Transitions { ..brul::gui::Transitions::DEFAULT },
    },
]);

fn classes(theme: Option<&str>) -> &'static [brul::gui::Class] {
    match theme {
        None | Some("dark") => &*CLASSES_DARK,
        Some("light") => &*CLASSES_LIGHT,
        Some(_) => &*CLASSES_DARK,
    }
}

#[allow(dead_code)]
fn styles() -> &'static brul::style::Stylesheet {
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

pub fn script() -> Option<&'static brul::script::Program> {
    None
}

pub fn names() -> &'static [&'static str] {
    &[]
}
"#;
    assert_eq!(views(emit::module(source).expect("module")), expected);
}

#[test]
fn emits_text_binding() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Text { name }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(r#"let widget0 = brul::gui::Widget::text("");"#));
    assert!(module.contains(
        r#"bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read_id(brul::VariableId::from_index(0))?; Ok(value) })));"#
    ));
}

#[test]
fn emits_a_dynamic_class_binding_and_its_table() {
    let source = r#"<style>
.tone { background: linear-gradient(135deg, #fb923c, #ef4444); }
</style>

<markup>
window Demo (title: "demo") {
    View (class: tone) { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");

    assert!(
        module.contains("brul::BindingTarget::Class { table: class_table(theme), expression: "),
        "{module}"
    );
    assert!(
        module.contains("fn class_none(name: &str) -> Option<brul::gui::Class>"),
        "{module}"
    );
    assert!(
        module.contains("brul::gui::style::class(styles(), name, None).ok()"),
        "{module}"
    );
    assert!(module.contains("class: \"tone\".into()"), "{module}");
}

#[test]
fn emits_no_class_table_without_a_dynamic_class() {
    let module = emit::module(
        r#"<style>
.tone { background: #ff0000; }
</style>

<markup>
window Demo (title: "demo") {
    View (tone) { }
}
</markup>
"#,
    )
    .expect("module");

    assert!(!module.contains("fn class_none"), "{module}");
}

#[test]
fn emits_vertical_alignment_for_a_text_state() {
    let source = r#"<style>
.a { width: 20px; height: 20px; vertical-align: top; }
.a:hover { vertical-align: center; }
</style>

<markup>
window Demo (title: "demo") {
    Text (a) { "hi" }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains("s::Kind::VerticalAlign"), "{module}");
    assert!(module.contains("state: Some(s::State::Hover)"), "{module}");
}

#[test]
fn emits_text_styles_for_a_state() {
    let source = r#"<style>
.a { width: 40px; height: 20px; font-weight: 500; }
.a:hover { font-weight: 600; }
</style>

<markup>
window Demo (title: "demo") {
    Text (a) { "hi" }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(
        module.contains("styled(brul::gui::Widget::text(\"hi\"), &[0], theme)"),
        "{module}"
    );
    assert!(
        module
            .contains("s::Property { kind: s::Kind::FontWeight, value: s::Value::Number(500.0) }"),
        "{module}"
    );
    assert!(
        module
            .contains("s::Property { kind: s::Kind::FontWeight, value: s::Value::Number(600.0) }"),
        "{module}"
    );
}

#[test]
fn emits_parameter_bindings() {
    let source = r#"<markup>
window Demo (title: "demo") {
    View {
        View (checked: on, selected: on, disabled: off, visible: shown) { }
        View (x: left, y: top) { }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    for target in ["Checked", "Selected", "Disabled", "Visible"] {
        assert!(
            module.contains(&format!("brul::BindingTarget::{target}(")),
            "no {target} binding"
        );
    }
    assert!(module.contains(
        r#"brul::BindingTarget::Offset { x: Some(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read_id(brul::VariableId::from_index(3))?; Ok(value) }"#
    ));
    assert!(
        module.contains(
            r#"y: Some(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read_id(brul::VariableId::from_index(4))?; Ok(value) }"#
        )
    );
}

#[test]
fn rejects_unknown_parameter() {
    let source = r#"<markup>
window Demo (title: "demo") {
    View (hovered: on) { }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(error.message, "parameter 'hovered' is not supported yet");
}

#[test]
fn emits_the_focusable_flag_and_focus_handlers() {
    let source = r#"<script>
    function gained() { }
    function lost() { }
</script>

<markup>
window Demo (title: "demo") {
    Row (focusable: true, on focus: gained, on blur: lost) { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(".focusable(true)"));
    assert!(module.contains(r#"HandlerEvent::Focus, "gained""#));
    assert!(module.contains(r#"HandlerEvent::Blur, "lost""#));
}

#[test]
fn emits_a_text_input_with_its_binding_and_handlers() {
    let source = r#"<script>
    function edited(ev) { }
    function submitted() { }
</script>

<markup>
window Demo (title: "demo") {
    TextInput (field, on input: edited, on submit: submitted) { draft }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains("brul::gui::Widget::input(\"\")"));
    assert!(module.contains("brul::BindingTarget::Input("));
    assert!(module.contains(r#"HandlerEvent::Input, "edited""#));
    assert!(module.contains(r#"HandlerEvent::Submit, "submitted""#));
}

#[test]
fn rejects_a_handler_that_ignores_the_event_arguments() {
    let source = r#"<script>
    function edited(value, caret) { }
</script>

<markup>
window Demo (title: "demo") {
    TextInput (field, on input: edited) { draft }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(
        error.message,
        "function 'edited' expects 1 arguments, got 2"
    );
}

#[test]
fn rejects_a_window_handler_that_ignores_the_key() {
    let source = r#"<script>
    function quit() { }
</script>

<markup>
window Demo (title: "demo", on key: quit) {
    View { }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(error.message, "function 'quit' expects 1 arguments, got 0");
}

#[test]
fn emits_a_multiline_text_input() {
    let source = r#"<markup>
window Demo (title: "demo") {
    TextInput (field, multiline: true) { draft }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(".multiline(true)"));
}

#[test]
fn rejects_a_multiline_that_is_not_a_literal() {
    let source = r#"<markup>
window Demo (title: "demo") {
    TextInput (field, multiline: draft) { draft }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(
        error.message,
        "parameter 'multiline' must be a boolean literal"
    );
}

#[test]
fn rejects_a_focusable_that_is_not_a_literal() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Row (focusable: shown) { }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(
        error.message,
        "parameter 'focusable' must be a boolean literal"
    );
}

#[test]
fn emits_widget_handlers() {
    let source = r#"<script>
    function toggle() { }
</script>

<markup>
window Demo (title: "demo") {
    Column (on drag: dragWindow) {
        Row (on click: toggle, on enter: toggle) { }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        r#"handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Click, "toggle"));"#
    ));
    assert!(module.contains(
        r#"handlers.push(brul::Handler::new(component, Some(widget0.id()), brul::HandlerEvent::Enter, "toggle"));"#
    ));
    assert!(module.contains("brul::gui::Widget::column(vec!["));
    assert!(module.contains(".draggable(true)"));
    assert!(!module.contains("HandlerEvent::Drag"));
}

#[test]
fn emits_if_structure() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        @if (shown) {
            Text { "a" }
        } @else {
            Text { "b" }
        }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "fn demo_if1_value0(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {"
    ));
    assert!(module.contains(
        "fn demo_if1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {\n    let mut bindings"
    ));
    assert!(module.contains("    let children = {\n"), "{module}");
    assert!(module.contains("brul::Value::Bool(condition) => condition,"));
    assert!(module.contains("        vec![\n"));
    assert!(module.contains(
        "brul::register_structure(build, component, widget0.id(), false, demo_if1_value0, demo_if1_rebuild);"
    ));
    assert!(
        module.contains(r#"brul::ViewError::expected("if condition", "Bool", other.type_name())"#)
    );
}

#[test]
fn emits_an_if_chain_as_one_structure() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        @if (shown) {
            Text { "a" }
        } @else if (pending) {
            Text { "b" }
        } @else {
            Text { "c" }
        }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains("fn demo_if1_value0("), "{module}");
    assert!(module.contains("fn demo_if1_value1("), "{module}");
    assert!(!module.contains("demo_if2_"), "{module}");
    assert_eq!(
        module.matches("fn demo_if1_rebuild(").count(),
        1,
        "{module}"
    );
    assert_eq!(
        module.matches("brul::register_structure(").count(),
        1,
        "{module}"
    );
    assert!(
        module.contains(
            "        } else {\n            let condition = demo_if1_value1(&mut component.borrow_mut())"
        ),
        "{module}"
    );
}

#[test]
fn emits_each_structure() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        @each (items as item, position (item.name)) { Text { item.name } }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "fn demo_each1_value(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {"
    ));
    assert!(module.contains(
        "fn demo_each1_rebuild(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle, old_rows: &[brul::RowInfo]) -> brul::Rebuilt {\n    let mut bindings"
    ));
    assert!(module.contains("component.borrow_mut().push_build_scope(vec!["));
    assert!(module.contains(
        "brul::Variable { name: brul::VariableId::from_index(1), value: item.clone() },"
    ));
    assert!(module.contains(
        r#"brul::Variable { name: brul::VariableId::from_index(2), value: brul::Value::Number(position as f64) },"#
    ));
    assert!(module.contains(
        "let key = demo_each1_key(&mut component.borrow_mut()).map(brul::RowKey::Explicit).unwrap_or(brul::RowKey::Index(position));"
    ));
    assert!(module.contains("let mut matcher = brul::RowMatcher::new(old_rows);"));
    assert!(module.contains("if let Some(other) = matcher.take(&key, &item, position) {"));
    assert!(module.contains("rows.push(brul::RowInfo { key, item, start, count, reuse: None });"));
    assert!(module.contains(
        "brul::register_structure(build, component, widget0.id(), true, demo_each1_value, demo_each1_rebuild);"
    ));
}

#[test]
fn emits_each_else_for_an_empty_collection() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        @each (items as item) { Text { item.name } } @else { Text { "empty" } }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    let empty = module
        .find("    if items.is_empty() {")
        .expect("empty test");
    let loop_ = module
        .find("    for (position, item) in items.into_iter().enumerate() {")
        .expect("collect loop");
    assert!(empty < loop_, "{module}");
    assert!(module.contains("        children.extend([\n"), "{module}");
    assert!(
        module.contains("brul::gui::Widget::text(\"empty\"),"),
        "{module}"
    );
    assert!(
        module.contains(
            "        rows.push(brul::RowInfo {\n            key: brul::RowKey::Index(0),\n            item: brul::Value::Array(std::sync::Arc::new(Vec::new())),\n            start: 0,\n            count: children.len(),\n            reuse: None,\n        });"
        ),
        "{module}"
    );
}

#[test]
fn emits_window_handler() {
    let source = r#"<script>
    function quit(ev) { }
</script>

<markup>
window Demo (title: "demo", on key: quit) {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        r#"handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::Key, "quit"));"#
    ));
}

#[test]
fn emits_a_window_close_handler() {
    let source = r#"<script>
    function hide() { }
</script>

<markup>
window Demo (title: "demo", on close: hide) {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        r#"handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::Close, "hide"));"#
    ));
}

#[test]
fn emits_a_window_position() {
    let source = r#"<markup>
window Demo (title: "demo", x: -40, y: 12.5) {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "        centered: false,\n        scroll: true,\n        position: Some(brul::WindowPosition { x: -40.0, y: 12.5 }),"
    ));

    let centered = r#"<markup>
window Demo (title: "demo", centered: true) {
    View { }
}
</markup>
"#;
    let module = emit::module(centered).expect("module");
    assert!(
        module.contains("        centered: true,\n        scroll: true,\n        position: None,")
    );
}

#[test]
fn rejects_invalid_events() {
    let unknown = r#"<markup>
window Demo (title: "demo") {
    View (on hovered: toggle) { }
}
</markup>
"#;
    assert_eq!(
        emit::module(unknown).expect_err("error").message,
        "event 'hovered' is not supported yet"
    );

    let missing = r#"<markup>
window Demo (title: "demo") {
    View (on click: toggle) { }
}
</markup>
"#;
    assert_eq!(
        emit::module(missing).expect_err("error").message,
        "function 'toggle' is not defined"
    );

    let key = r#"<markup>
window Demo (title: "demo") {
    View (on key: quit) { }
}
</markup>
"#;
    assert_eq!(
        emit::module(key).expect_err("error").message,
        "key events are only supported on the window"
    );

    let close = r#"<markup>
window Demo (title: "demo") {
    View (on close: quit) { }
}
</markup>
"#;
    assert_eq!(
        emit::module(close).expect_err("error").message,
        "close events are only supported on the window"
    );

    let window = r#"<markup>
window Demo (title: "demo", on click: quit) {
    View { }
}
</markup>
"#;
    assert_eq!(
        emit::module(window).expect_err("error").message,
        "window event 'click' is not supported yet"
    );
}

fn emitted(source: &str) -> String {
    emit::expression(&brul_syntax::parser::parse_expression(source).expect("expression parses"))
}

#[test]
fn emits_window_config() {
    let source = r#"<markup>
window Demo (title: "demo", width: 488, height: 800, frameless: true, transparent: true) {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "pub fn demo_window() -> brul::WindowConfig {\n    brul::WindowConfig {\n        title: \"demo\".to_owned(),\n        width: 488,\n        height: 800,\n        frameless: true,\n        transparent: true,\n        visible: true,\n        centered: false,\n        scroll: true,\n        position: None,\n        ..brul::WindowConfig::default()\n    }\n}"
    ));
}

#[test]
fn emits_every_window_with_its_flags() {
    let source = r#"<markup>
window Picker (title: "picker", width: 488, height: 800, frameless: true, transparent: true) {
    View { }
}

window Settings (title: "settings", width: 760, height: 560, frameless: false) {
    Text { "settings" }
}
</markup>
"#;
    let module = emit::module(source).expect("module");

    assert!(module.contains("pub fn settings_view(theme: Option<ThemeId>"));
    assert!(module.contains(
        "pub fn settings_window() -> brul::WindowConfig {\n    brul::WindowConfig {\n        title: \"settings\".to_owned(),\n        width: 760,\n        height: 560,\n        frameless: false,\n        transparent: false,\n        visible: true,\n        centered: false,\n        scroll: true,\n        position: None,"
    ));
    assert!(module.contains("name: \"Picker\", config: picker_window(), view: picker_view }"));
    assert!(
        module.contains("name: \"Settings\", config: settings_window(), view: settings_view }")
    );
}

#[test]
fn rejects_two_windows_with_the_same_name() {
    let source = r#"<markup>
window Picker (title: "one") {
    View { }
}

window Picker (title: "two") {
    View { }
}
</markup>
"#;
    assert_eq!(
        emit::module(source).expect_err("error").message,
        "duplicate window 'Picker'"
    );
}

#[test]
fn emits_script_program() {
    let source = r#"<script>
    const limit = 2;
    let count = 0;

    function bump(step) {
        if (step > limit) {
            count = limit;
        } else {
            count = count + step;
        }
    }

    function closePanel() {
        count = 0;
    }
</script>

<markup>
window Demo (title: "demo") {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains("pub fn script() -> Option<&'static brul::script::Program> {"));
    assert!(module.contains("const SOURCE: &str = r#\""));
    assert!(module.contains("const limit = 2;"));
    assert!(module.contains("brul::script::parse(&text)"));
    assert!(!module.contains("brul::script::Statement::"));
    assert!(module.contains("#[allow(dead_code)]\npub struct Script(brul::Handle);"));
    assert!(module.contains(
        "pub fn bump(&self, step: impl Into<brul::Value>) {\n        self.0.call_with(\"bump\", ::std::vec![step.into()]);\n    }"
    ));
    assert!(
        module.contains("pub fn close_panel(&self) {\n        self.0.call(\"closePanel\");\n    }")
    );
}

#[test]
fn emits_literal_and_ident_expressions() {
    assert_eq!(
        emitted("x"),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::Ident("x".to_owned()), brul::ast::Position { line: 1, column: 1 })"#
    );
    assert_eq!(
        emitted("42"),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::Number(42.0), brul::ast::Position { line: 1, column: 1 })"#
    );
    assert_eq!(
        emitted("\"hi\""),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::String("hi".to_owned()), brul::ast::Position { line: 1, column: 1 })"#
    );
    assert_eq!(
        emitted("true"),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::Bool(true), brul::ast::Position { line: 1, column: 1 })"#
    );
    assert_eq!(
        emitted("null"),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::Null, brul::ast::Position { line: 1, column: 1 })"#
    );
}

#[test]
fn emits_nested_expressions() {
    assert_eq!(
        emitted("a.b + 1"),
        r#"brul::ast::Expr::new(brul::ast::ExprKind::Binary { op: brul::ast::BinaryOp::Add, left: Box::new(brul::ast::Expr::new(brul::ast::ExprKind::Field { target: Box::new(brul::ast::Expr::new(brul::ast::ExprKind::Ident("a".to_owned()), brul::ast::Position { line: 1, column: 1 })), name: "b".to_owned() }, brul::ast::Position { line: 1, column: 1 })), right: Box::new(brul::ast::Expr::new(brul::ast::ExprKind::Number(1.0), brul::ast::Position { line: 1, column: 7 })) }, brul::ast::Position { line: 1, column: 1 })"#
    );
}

#[test]
fn emits_every_expression_kind() {
    let cases = [
        ("a[0]", "brul::ast::ExprKind::Index"),
        ("f(1, 2)", "brul::ast::ExprKind::Call"),
        ("!x", "brul::ast::ExprKind::Unary"),
        ("a ? b : c", "brul::ast::ExprKind::Ternary"),
        ("a = b", "brul::ast::ExprKind::Assign"),
        ("[1, 2]", "brul::ast::ExprKind::Array"),
        ("{ a: 1 }", "brul::ast::ExprKind::Object"),
        ("`v=${a}`", "brul::ast::ExprKind::Template"),
    ];
    for (source, kind) in cases {
        assert!(
            emitted(source).contains(kind),
            "'{source}' did not emit {kind}"
        );
    }
}

fn operated(source: &str) -> String {
    let expression = brul_syntax::parser::parse_expression(source).expect("expression parses");
    let names = emit::identifiers(&expression);
    emit::operation(&expression, &names)
}

#[test]
fn emits_value_literals_and_reads() {
    assert_eq!(
        operated("x"),
        "component.read_id(brul::VariableId::from_index(0))?"
    );
    assert_eq!(operated("42"), "brul::Value::Number(42.0)");
    assert_eq!(operated("\"hi\""), "brul::Value::String(\"hi\".into())");
    assert_eq!(operated("true"), "brul::Value::Bool(true)");
    assert_eq!(operated("null"), "brul::Value::Null");
    assert_eq!(
        operated("a.b"),
        "brul::eval::read_field(&(component.read_id(brul::VariableId::from_index(0))?), \"b\")?"
    );
    assert_eq!(
        operated("a[0]"),
        "brul::eval::read_index(&(component.read_id(brul::VariableId::from_index(0))?), &(brul::Value::Number(0.0)))?"
    );
}

#[test]
fn emits_value_operations() {
    assert_eq!(
        operated("!x"),
        "brul::eval::apply_unary(brul::ast::UnaryOp::Not, component.read_id(brul::VariableId::from_index(0))?)?"
    );
    assert_eq!(
        operated("1 + 2"),
        "brul::eval::apply_binary(brul::ast::BinaryOp::Add, brul::Value::Number(1.0), brul::Value::Number(2.0))?"
    );
    assert_eq!(
        operated("[1, x]"),
        "brul::Value::Array(vec![brul::Value::Number(1.0), component.read_id(brul::VariableId::from_index(0))?].into())"
    );
    assert_eq!(
        operated("{ a: 1 }"),
        "brul::Value::Object(vec![(\"a\".to_owned(), brul::Value::Number(1.0))].into())"
    );
    assert_eq!(
        operated("`v=${x}`"),
        "brul::Value::String(brul::eval::template(vec![brul::Value::String(\"v=\".into()), component.read_id(brul::VariableId::from_index(0))?])?)"
    );
}

#[test]
fn emits_value_calls() {
    assert_eq!(
        operated("f(1)"),
        "brul::eval::call(component, \"f\", vec![brul::Value::Number(1.0)])?"
    );
    assert!(
        operated("every(100, tick)").contains("component.every(interval, \"tick\")?"),
        "{}",
        operated("every(100, tick)")
    );
    assert!(operated("every(100)").contains("brul::Error::Arity"));
    assert!(operated("every(100, 5)").contains("expected: \"function name\""));
    assert!(operated("a.b()").contains("Unsupported(\"method call\")"));
    assert!(operated("a ? b : c").contains("expected: \"Bool\""));
    assert!(!operated("a ? b : c").contains("match ("));
    assert!(!operated("every(100, tick)").contains("match ("));
}

#[test]
fn emits_value_assignments() {
    let source = operated("a.b = 1");
    assert!(
        source.contains("let value = brul::Value::Number(1.0);"),
        "{source}"
    );
    assert!(
        source
            .contains("let mut container0 = component.read_id(brul::VariableId::from_index(0))?;"),
        "{source}"
    );
    assert!(
        source.contains("brul::eval::assign_field(&mut container0, \"b\", value.clone())?;"),
        "{source}"
    );
    assert!(
        source
            .contains("component.assign_id(brul::VariableId::from_index(0), container0.clone())?;"),
        "{source}"
    );
    assert!(source.ends_with("value }"), "{source}");

    assert!(
        operated("items[0] = x").contains("brul::eval::assign_index("),
        "{}",
        operated("items[0] = x")
    );
    assert!(
        operated("x += 1").contains(
            "brul::ast::BinaryOp::Add, component.read_id(brul::VariableId::from_index(0))?"
        ),
        "{}",
        operated("x += 1")
    );
    assert!(operated("x = [1, 2]").contains("brul::Value::Array"));
}

fn scratch(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("brul-emit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

#[test]
fn emits_imported_component() {
    let dir = scratch(
        "component",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(module.contains("pub fn badge_view(theme: Option<&str>"));
    assert!(module.contains("pub fn badge_script() -> Option<&'static brul::script::Program>"));
    assert!(module.contains("let built = brul::instance(build, component, brul::Instance {"));
    assert!(module.contains("bindings.extend(built.bindings);"));
    assert!(module.contains("props.extend(built.props);"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn resolves_an_import_that_declares_imports_of_its_own() {
    let dir = scratch(
        "nested-import",
        &[
            (
                "label.brul",
                "<markup>\ncomponent Label {\n    Text { \"label\" }\n}\n</markup>\n",
            ),
            (
                "badge.brul",
                "<use>\n    import Label from \"./label.brul\";\n</use>\n\n<markup>\ncomponent Badge {\n    Label { }\n}\n</markup>\n",
            ),
            (
                "other.brul",
                "<markup>\ncomponent Other {\n    prop note: String = \"\";\n    Text { note }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n    import Other from \"./other.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Other (note: \"hi\") { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(module.contains("pub fn other_view"), "{module}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn splits_every_component_into_its_own_file() {
    let dir = scratch(
        "split",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::split_module_file(&dir.join("window.brul")).unwrap();
    assert_eq!(module.files.len(), 1);
    let badge = &module.files[0];
    assert_eq!(badge.name, "bc_badge");
    assert!(badge.source.starts_with("use super::*;\n"));
    assert!(badge.source.contains("pub fn badge_view"));
    assert!(!badge.source.contains("pub fn demo_view("));
    assert!(module.root.contains("pub mod bc_badge {"));
    assert!(module.root.contains("pub use self::bc_badge::*;"));
    assert!(module.root.contains("pub fn demo_view("));
    assert_eq!(
        module.flat(),
        emit::module_file(&dir.join("window.brul")).unwrap()
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_component_properties() {
    let dir = scratch(
        "properties",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    prop label: String = \"badge\";\n    prop size: Number = 10;\n    Text { label }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge (label: \"hi\") { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(module.contains("name: \"Badge\","));
    assert!(module.contains(
        "brul::PropArgument { name: brul::VariableId::from_index(0), ty: brul::PropType::String, expression: |component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = brul::Value::String(\"hi\".into()); Ok(value) }"
    ));
    assert!(module.contains(
        "brul::PropArgument { name: brul::VariableId::from_index(1), ty: brul::PropType::Number,"
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_component_properties() {
    let cases = [
        (
            "Badge (nope: 1) { }",
            "component 'Badge' has no property 'nope'",
        ),
        (
            "Badge (label: \"a\", label: \"b\") { }",
            "property 'label' is given twice",
        ),
    ];
    for (markup, expected) in cases {
        let dir = scratch(
            "properties-error",
            &[
                (
                    "badge.brul",
                    "<markup>\ncomponent Badge {\n    prop label: String = \"badge\";\n    Text { label }\n}\n</markup>\n",
                ),
                (
                    "window.brul",
                    &format!(
                        "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {{\n    Column {{\n        {markup}\n    }}\n}}\n</markup>\n"
                    ),
                ),
            ],
        );
        let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
        assert_eq!(error.message, expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn emits_component_events() {
    let dir = scratch(
        "events",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    event clicked(i: Number);\n    Text { \"badge\" }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<script>\n    function launch(i) {\n    }\n</script>\n\n<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge (on clicked: launch) { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(module.contains(
        "brul::ComponentEvent { name: \"clicked\", function: \"launch\", params: &[brul::PropType::Number], component: std::rc::Rc::clone(component) },"
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_component_events() {
    let cases = [
        (
            "Badge (on nope: launch) { }",
            "component 'Badge' has no event 'nope'",
        ),
        (
            "Badge (on clicked: missing) { }",
            "function 'missing' is not defined",
        ),
    ];
    for (markup, expected) in cases {
        let dir = scratch(
            "events-error",
            &[
                (
                    "badge.brul",
                    "<markup>\ncomponent Badge {\n    event clicked(i: Number);\n    Text { \"badge\" }\n}\n</markup>\n",
                ),
                (
                    "window.brul",
                    &format!(
                        "<script>\n    function launch() {{\n    }}\n</script>\n\n<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {{\n    Column {{\n        {markup}\n    }}\n}}\n</markup>\n"
                    ),
                ),
            ],
        );
        let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
        assert_eq!(error.message, expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn rejects_component_children() {
    let dir = scratch(
        "children",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { Text { \"x\" } }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert_eq!(error.message, "component 'Badge' does not accept children");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_component_slot() {
    let dir = scratch(
        "slot",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    View {\n        Row {\n            @slot\n        }\n    }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { Text { \"x\" } }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("brul::slot(theme, build, brul::DEFAULT_SLOT)?;"),
        "{module}"
    );
    assert!(
        module.contains("fn demo_slot1(theme: Option<&str>"),
        "{module}"
    );
    assert!(
        module.contains("children.push(brul::gui::Widget::text(\"x\"));"),
        "{module}"
    );
    assert!(
        module.contains("brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot1 },"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_an_empty_slot_without_children() {
    let dir = scratch(
        "empty-slot",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    View {\n        @slot\n    }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(module.contains("slots: &[],"), "{module}");
    assert!(!module.contains("demo_slot1"), "{module}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_a_slot_inside_a_structure() {
    let dir = scratch(
        "slot-structure",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    prop shown: Bool = false;\n    View {\n        @if (shown) {\n            @slot\n        }\n    }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge { Text { \"x\" } }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("let mut children = Vec::new();"),
        "{module}"
    );
    assert!(
        module.contains("children.extend(built.widget.into_children());"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_a_slot_outside_a_component() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        @slot
    }
}
</markup>
"#;
    let error = emit::module(source).unwrap_err();
    assert_eq!(error.message, "slot is only allowed inside a component");
}

#[test]
fn emits_named_slots() {
    let dir = scratch(
        "named-slots",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    View {\n        @slot(header)\n        @slot\n    }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge {\n            @slot(header) { Text { \"head\" } }\n            Text { \"body\" }\n        }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("brul::slot(theme, build, \"header\")?;"),
        "{module}"
    );
    assert!(
        module.contains("brul::slot(theme, build, brul::DEFAULT_SLOT)?;"),
        "{module}"
    );
    assert!(
        module.contains("brul::SlotArgument { name: brul::DEFAULT_SLOT, view: demo_slot1 },"),
        "{module}"
    );
    assert!(
        module.contains("brul::SlotArgument { name: \"header\", view: demo_slot2 },"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_malformed_slot_fills() {
    let cases = [
        (
            "<markup>\ncomponent Badge {\n    View {\n        @slot(header)\n    }\n}\n</markup>\n",
            "Badge { Text { \"x\" } }",
            "component 'Badge' has no default slot",
        ),
        (
            "<markup>\ncomponent Badge {\n    View {\n        @slot\n    }\n}\n</markup>\n",
            "Badge { @slot(header) { } }",
            "component 'Badge' has no slot 'header'",
        ),
        (
            "<markup>\ncomponent Badge {\n    View {\n        @slot(header)\n    }\n}\n</markup>\n",
            "Badge { @slot(header) { } @slot(header) { } }",
            "slot 'header' is filled twice",
        ),
        (
            "<markup>\ncomponent Badge {\n    View {\n        @slot\n    }\n}\n</markup>\n",
            "Badge { @slot { Text { \"x\" } } }",
            "fill the default slot with plain children",
        ),
    ];
    for (badge, call, expected) in cases {
        let dir = scratch(
            "slot-error",
            &[
                ("badge.brul", badge),
                (
                    "window.brul",
                    &format!(
                        "<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {{\n    Column {{\n        {call}\n    }}\n}}\n</markup>\n"
                    ),
                ),
            ],
        );
        let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
        assert_eq!(error.message, expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn emits_component_classes() {
    let dir = scratch(
        "component-classes",
        &[
            (
                "badge.brul",
                "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
            ),
            (
                "window.brul",
                "<style>\n.primary { width: 10px; }\n.tone { width: 20px; }\n</style>\n\n<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge (primary, class: \"tone\") { }\n    }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("styled(built.widget, &[0], theme);"),
        "{module}"
    );
    assert!(
        module.contains("brul::BindingTarget::Class { table: class_table(theme), expression:"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_the_kit_components() {
    let dir = scratch("kit", &[]);
    std::fs::write(
        dir.join("window.brul"),
        r#"<script>
    let shown = false;
    let draft = "";
</script>

<style>
    theme dark {
        surface: #1e293b;
        surface-hover: #273549;
        accent: #3b82f6;
        accent-hover: #2563eb;
        text: #f8fafc;
        border: #334155;
        muted: #cbd5e1;
    }
    theme light {
        surface: #f1f5f9;
        surface-hover: #e2e8f0;
        accent: #2563eb;
        accent-hover: #1d4ed8;
        text: #0f172a;
        border: #cbd5e1;
        muted: #475569;
    }
</style>

<use>
    import Button from "brul:button";
    import Card from "brul:card";
    import Tooltip from "brul:tooltip";
    import Checkbox from "brul:checkbox";
    import Dialog from "brul:dialog";
</use>

<markup>
window Demo (title: "demo") {
    Column {
        Button (label: "Open", variant: "primary") { }
        Card {
            @slot(header) { Text { "header" } }
            Text { "body" }
            @slot(footer) { Text { "footer" } }
        }
        Tooltip (text: "tip") { Text { "trigger" } }
        Checkbox (label: "Remember") { }
        Dialog (open: shown, title: "Edit") {
            TextInput (name) { draft }
        }
    }
}
</markup>
"#,
    )
    .unwrap();
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    for name in ["button", "card", "tooltip", "checkbox", "dialog"] {
        assert!(
            module.contains(&format!("pub fn {name}_view(")),
            "{name}: {module}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_unknown_kit_component_lists_the_kit() {
    let dir = scratch(
        "kit-unknown",
        &[(
            "window.brul",
            "<use>\n    import Button from \"brul:buton\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column { Button (label: \"x\") { } }\n}\n</markup>\n",
        )],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert!(
        error.message.contains("unknown kit component 'buton'"),
        "{}",
        error.message
    );
    assert!(error.message.contains("button"), "{}", error.message);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn imported_components_inherit_the_root_palette() {
    let dir = scratch(
        "palette",
        &[
            (
                "chip.brul",
                "<markup>\ncomponent Chip {\n    View (chip) { }\n}\n</markup>\n\n<style>\n.chip { background: theme.surface; }\n</style>\n",
            ),
            (
                "window.brul",
                "<style>\ntheme dark {\n    surface: #101010;\n}\ntheme light {\n    surface: #ffffff;\n}\n</style>\n\n<use>\n    import Chip from \"./chip.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Chip { }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert_eq!(
        module.matches("s::Theme { name: \"dark\".into()").count(),
        2,
        "{module}"
    );
    assert!(
        module.contains("s::Value::ThemeRef(\"surface\".into())"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_unknown_theme_key_in_an_imported_component_names_the_file() {
    let dir = scratch(
        "palette-unknown",
        &[
            (
                "chip.brul",
                "<markup>\ncomponent Chip {\n    View (chip) { }\n}\n</markup>\n\n<style>\n.chip { background: theme.danger; }\n</style>\n",
            ),
            (
                "window.brul",
                "<style>\ntheme dark {\n    surface: #101010;\n}\n</style>\n\n<use>\n    import Chip from \"./chip.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Chip { }\n}\n</markup>\n",
            ),
        ],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert!(error.message.contains("chip.brul"), "{}", error.message);
    assert!(
        error.message.contains("unknown theme key 'danger'"),
        "{}",
        error.message
    );
    assert_eq!(error.line, 8);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_imported_component_cannot_declare_a_theme() {
    let dir = scratch(
        "palette-local",
        &[
            (
                "chip.brul",
                "<markup>\ncomponent Chip {\n    View (chip) { }\n}\n</markup>\n\n<style>\ntheme dark {\n    surface: #ffffff;\n}\n</style>\n",
            ),
            (
                "window.brul",
                "<style>\ntheme dark {\n    surface: #101010;\n}\n</style>\n\n<use>\n    import Chip from \"./chip.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Chip { }\n}\n</markup>\n",
            ),
        ],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert!(error.message.contains("chip.brul"), "{}", error.message);
    assert!(
        error
            .message
            .contains("a theme can only be declared in the root <style>"),
        "{}",
        error.message
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_override_follows_the_component_rules() {
    let dir = scratch(
        "override",
        &[
            (
                "chip.brul",
                "<markup>\ncomponent Chip {\n    View (chip) { }\n}\n</markup>\n\n<style>\n.chip { background: #101010; }\n</style>\n",
            ),
            (
                "window.brul",
                "<style>\noverride Chip {\n    .chip { background: #ffffff; }\n    .chip:hover { background: #efefef; }\n}\n</style>\n\n<use>\n    import Chip from \"./chip.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Chip { }\n}\n</markup>\n",
            ),
        ],
    );
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    let own = module
        .find("s::Color::rgba(16, 16, 16, 255)")
        .unwrap_or_else(|| panic!("no component rule: {module}"));
    let over = module
        .find("s::Color::rgba(255, 255, 255, 255)")
        .unwrap_or_else(|| panic!("no override rule: {module}"));
    assert!(own < over, "{module}");
    assert!(module.contains("state: Some(s::State::Hover)"), "{module}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_override_of_a_component_that_is_not_imported_is_an_error() {
    let dir = scratch(
        "override-missing-component",
        &[(
            "window.brul",
            "<style>\noverride Chip {\n    .chip { background: #ffffff; }\n}\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View { }\n}\n</markup>\n",
        )],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert!(
        error
            .message
            .contains("override names 'Chip', which is not imported"),
        "{}",
        error.message
    );
    assert_eq!(error.line, 2);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_override_of_an_unknown_class_is_an_error() {
    let dir = scratch(
        "override-missing-class",
        &[
            (
                "chip.brul",
                "<markup>\ncomponent Chip {\n    View (chip) { }\n}\n</markup>\n\n<style>\n.chip { background: #101010; }\n</style>\n",
            ),
            (
                "window.brul",
                "<style>\noverride Chip {\n    .chip-label { background: #ffffff; }\n}\n</style>\n\n<use>\n    import Chip from \"./chip.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Chip { }\n}\n</markup>\n",
            ),
        ],
    );
    let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
    assert!(
        error
            .message
            .contains("component 'Chip' has no class '.chip-label'"),
        "{}",
        error.message
    );
    assert_eq!(error.line, 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lists_imported_files_outside_the_entry_directory() {
    let dir = std::env::temp_dir().join(format!("brul-emit-sources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("shared")).unwrap();
    std::fs::write(
        dir.join("shared/badge.brul"),
        "<markup>\ncomponent Badge {\n    Text { \"badge\" }\n}\n</markup>\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("app/window.brul"),
        "<use>\n    import Badge from \"../shared/badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Badge { }\n}\n</markup>\n",
    )
    .unwrap();
    let module = emit::split_module_file(&dir.join("app/window.brul")).unwrap();
    let badge = std::fs::canonicalize(dir.join("shared/badge.brul")).unwrap();
    let entry = std::fs::canonicalize(dir.join("app/window.brul")).unwrap();
    assert!(module.sources.contains(&badge), "{:?}", module.sources);
    assert!(module.sources.contains(&entry), "{:?}", module.sources);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_names_in_first_appearance_order() {
    let source = r#"<script>
    let selected = 0;
    function bump(step) {
        selected = selected + step;
    }
</script>

<markup>
window Demo (title: "demo") {
    Column {
        @each (browsers as browser, index (browser.name)) {
            Text { browser.name }
        }
        Text { selected }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "pub fn names() -> &'static [&'static str] {\n    &[\"browsers\", \"browser\", \"index\", \"selected\", \"step\"]\n}"
    ));
}

const PNG_SIGNATURE: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[test]
fn emits_a_linear_gradient_background() {
    let source = "<style>\n.a { background: linear-gradient(90deg, #ff0000, #0000ff); }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (a) { }\n}\n</markup>\n";
    let module = emit::module(source).unwrap();
    assert!(
        module.contains(
            "s::Value::Gradient(s::Gradient { angle: 90.0, from: s::ColorValue::Color(s::Color::rgba(255, 0, 0, 255)), to: s::ColorValue::Color(s::Color::rgba(0, 0, 255, 255)) })"
        ),
        "{module}"
    );
}

#[test]
fn emits_color_expressions() {
    let source = "<style>\ntheme light { accent: #3b82f6; }\n.a { background: alpha(theme.accent, 50%); border-color: mix(#3b82f6, white, 20%); }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (a) { }\n}\n</markup>\n";
    let module = emit::module(source).unwrap();
    assert!(
        module.contains(
            "s::Value::ColorExpr(s::ColorExpr::Alpha { color: s::ColorValue::Theme(\"accent\".into()), amount: 50.0 })"
        ),
        "{module}"
    );
    assert!(
        module.contains(
            "s::Value::ColorExpr(s::ColorExpr::Mix { from: s::ColorValue::Color(s::Color::rgba(59, 130, 246, 255)), to: s::ColorValue::Color(s::Color::rgba(255, 255, 255, 255)), amount: 20.0 })"
        ),
        "{module}"
    );
}

#[test]
fn emits_resource_and_image_with_tint() {
    let dir = scratch(
        "resource",
        &[(
            "window.brul",
            r#"<style>
.icon { tint: #336699; }
</style>

<use>
    resource check from "./check.png";
</use>

<markup>
window Demo (title: "demo") {
    Image (icon) { check }
}
</markup>
"#,
        )],
    );
    std::fs::write(dir.join("check.png"), PNG_SIGNATURE).unwrap();
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("pub fn resource_check() -> &'static [u8]"),
        "{module}"
    );
    assert!(module.contains("include_bytes!("), "{module}");
    assert!(
        module.contains("brul::gui::Widget::image(brul::gui::Image::from_bytes(resource_check())"),
        "{module}"
    );
    assert!(
        module.contains("s::Property { kind: s::Kind::Tint, value: s::Value::Color(s::Color::rgba(51, 102, 153, 255)) }"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_a_svg_resource() {
    let dir = scratch(
        "svg-resource",
        &[(
            "window.brul",
            r#"<use>
    resource icon from "./icon.svg";
</use>

<markup>
window Demo (title: "demo") {
    Image (icon) { icon }
}
</markup>
"#,
        )],
    );
    std::fs::write(
        dir.join("icon.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#,
    )
    .unwrap();
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("brul::gui::Widget::image(brul::gui::Image::from_bytes(resource_icon())"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_component_resource_with_component_prefix() {
    let dir = scratch(
        "component-resource",
        &[
            (
                "badge.brul",
                r#"<use>
    resource check from "./check.png";
</use>

<markup>
component Badge {
    Image (icon) { check }
}
</markup>
"#,
            ),
            (
                "window.brul",
                r#"<use>
    import Badge from "./badge.brul";
</use>

<markup>
window Demo (title: "demo") {
    Badge { }
}
</markup>
"#,
            ),
        ],
    );
    std::fs::write(dir.join("check.png"), PNG_SIGNATURE).unwrap();
    let module = emit::module_file(&dir.join("window.brul")).unwrap();
    assert!(
        module.contains("pub fn badge_resource_check() -> &'static [u8]"),
        "{module}"
    );
    assert!(
        module.contains("brul::gui::Image::from_bytes(badge_resource_check())"),
        "{module}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_broken_resources() {
    let cases = [
        ("no file", "", "cannot read resource './check.png'"),
        (
            "not png",
            "plain text",
            "resource './check.png' is neither PNG nor SVG",
        ),
        ("duplicate", "", "resource 'check' is defined twice"),
    ];
    for (name, contents, expected) in cases {
        let use_block = if name == "duplicate" {
            "    resource check from \"./check.png\";\n    resource check from \"./check.png\";\n"
        } else {
            "    resource check from \"./check.png\";\n"
        };
        let dir = scratch(
            name,
            &[(
                "window.brul",
                &format!(
                    "<use>\n{use_block}</use>\n\n<markup>\nwindow Demo (title: \"demo\") {{\n    Image {{ check }}\n}}\n</markup>\n"
                ),
            )],
        );
        match name {
            "not png" => std::fs::write(dir.join("check.png"), contents).unwrap(),
            "duplicate" => std::fs::write(dir.join("check.png"), PNG_SIGNATURE).unwrap(),
            _ => {}
        }
        let error = emit::module_file(&dir.join("window.brul")).unwrap_err();
        assert!(
            error.message.contains(expected),
            "{name}: {}",
            error.message
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn emits_image_value_binding() {
    let source =
        "<markup>\nwindow Demo (title: \"demo\") {\n    Image (icon) { icon }\n}\n</markup>\n";
    let module = emit::module(source).unwrap();
    assert!(
        module.contains("brul::gui::Widget::image(brul::gui::Texture::empty())"),
        "{module}"
    );
    assert!(module.contains("brul::BindingTarget::Image("), "{module}");
}

#[test]
fn emits_clip_only_when_it_is_set() {
    let clipped = "<style>\n.panel { clip: true; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (panel) { }\n}\n</markup>\n";
    let module = emit::module(clipped).unwrap();
    assert!(
        module.contains("kind: s::Kind::Clip, value: s::Value::Bool(true)"),
        "{module}"
    );

    let plain = "<style>\n.panel { border-radius: 4px; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (panel) { }\n}\n</markup>\n";
    let module = emit::module(plain).unwrap();
    assert!(!module.contains("s::Kind::Clip"), "{module}");
}

#[test]
fn emits_scroll_only_when_it_is_set() {
    let scrolling = "<style>\n.list { scroll: true; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (list) { }\n}\n</markup>\n";
    let module = emit::module(scrolling).unwrap();
    assert!(
        module.contains("kind: s::Kind::Scroll, value: s::Value::Bool(true)"),
        "{module}"
    );

    let plain = "<style>\n.list { border-radius: 4px; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (list) { }\n}\n</markup>\n";
    let module = emit::module(plain).unwrap();
    assert!(!module.contains("s::Kind::Scroll"), "{module}");
}

#[test]
fn emits_geometry_shadow_and_text_properties_in_a_static_class() {
    let source = r#"<style>
.a {
    border-radius: 8px;
    scroll: true;
    box-shadow-offset-x: 2px;
    box-shadow-offset-y: -3px;
    box-shadow-blur: 4px;
    font-size: 18px;
    font-family: "Inter";
}
</style>

<markup>
window Styled (title: "styled") {
    View (a) { }
}
</markup>
"#;
    let module = emit::module(source).unwrap();
    assert!(module.contains("border_radius: Some(8)"), "{module}");
    assert!(module.contains("scroll: Some(true)"), "{module}");
    assert!(module.contains("shadow_x: Some(2)"), "{module}");
    assert!(module.contains("shadow_y: Some(-3)"), "{module}");
    assert!(module.contains("shadow_blur: Some(4)"), "{module}");
    assert!(module.contains("size: Some(18.0)"), "{module}");
    assert!(
        module.contains("family: Some(String::from(\"Inter\"))"),
        "{module}"
    );
}

#[test]
fn resource_without_a_file_path_is_rejected() {
    let source = "<use>\n    resource check from \"./check.png\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Image { check }\n}\n</markup>\n";
    let error = emit::module(source).unwrap_err();
    assert!(
        error
            .message
            .contains("cannot resolve resource './check.png': source has no file path"),
        "{}",
        error.message
    );
}

#[test]
fn emits_a_theme_id_for_the_root_palette() {
    let source = "<style>\ntheme dark { accent: #6c8cff; }\ntheme high-contrast { accent: #ffffff; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View { }\n}\n</markup>\n";
    let module = emit::module(source).expect("module");
    assert!(
        module.contains("pub enum ThemeId {\n    Dark,\n    HighContrast,\n}"),
        "{module}"
    );
    assert!(
        module.contains("            Self::HighContrast => \"high-contrast\","),
        "{module}"
    );
    assert!(
        module.contains(
            "    pub const ALL: &'static [ThemeId] = &[\n        ThemeId::Dark,\n        ThemeId::HighContrast,\n    ];"
        ),
        "{module}"
    );
}

#[test]
fn a_palette_without_themes_emits_an_empty_theme_id() {
    let source = "<markup>\nwindow Demo (title: \"demo\") {\n    View { }\n}\n</markup>\n";
    let module = emit::module(source).expect("module");
    assert!(module.contains("pub enum ThemeId {\n}"), "{module}");
    assert!(module.contains("        match self {}\n"), "{module}");
}

#[test]
fn themes_that_share_a_variant_are_an_error() {
    let source = "<style>\ntheme dark { accent: #6c8cff; }\ntheme Dark { accent: #ffffff; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View { }\n}\n</markup>\n";
    let error = emit::module(source).unwrap_err();
    assert!(
        error.message.contains("share the variant ThemeId::Dark"),
        "{}",
        error.message
    );
}

#[test]
fn emits_a_layer() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Layer {
        Text { "x" }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(
        module.contains("brul::gui::Widget::layer(vec!["),
        "{module}"
    );
}

#[test]
fn emits_a_modal_layer_that_dismisses() {
    let source = r#"<script>
    function dismiss() { }
</script>

<markup>
window Demo (title: "demo") {
    Layer (modal: true, on dismiss: dismiss) {
        Text { "x" }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(".modal(true)"), "{module}");
    assert!(
        module.contains("brul::HandlerEvent::Dismiss, \"dismiss\""),
        "{module}"
    );
}

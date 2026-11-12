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
    let expected = r#"#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn view(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {
    component.borrow_mut().set_names(names());
    brul::gui::style::check_theme(styles(), theme).map_err(|error| error.to_string())?;
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = brul::gui::Widget::row(vec![
                {
                    let widget1 = brul::gui::Widget::text("a");
                    widget1
                },
                {
                    let widget2 = brul::gui::Widget::column(vec![]);
                    widget2
                },
            ]);
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

fn styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![],
    });
    &STYLES
}

fn styled(
    widget: brul::gui::Widget,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<brul::gui::Widget, String> {
    brul::gui::style::styled(widget, styles(), classes, theme)
        .map_err(|error| error.to_string())
}

pub fn window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "demo".to_owned(),
        width: 960,
        height: 640,
        frameless: false,
        transparent: false,
        ..brul::WindowConfig::default()
    }
}

pub fn windows() -> Vec<brul::WindowDef> {
    vec![
        brul::WindowDef { name: "Demo", config: window(), view },
    ]
}

pub fn script() -> Option<brul::script::Program> {
    None
}

pub fn names() -> &'static [&'static str] {
    &[]
}
"#;
    assert_eq!(emit::module(source).expect("module"), expected);
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
    let expected = r#"#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn view(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {
    component.borrow_mut().set_names(names());
    brul::gui::style::check_theme(styles(), theme).map_err(|error| error.to_string())?;
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let widget0 = styled(brul::gui::Widget::column(vec![]), &["box"], theme)?;
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
    widget: brul::gui::Widget,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<brul::gui::Widget, String> {
    brul::gui::style::styled(widget, styles(), classes, theme)
        .map_err(|error| error.to_string())
}

pub fn window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "styled".to_owned(),
        width: 960,
        height: 640,
        frameless: false,
        transparent: false,
        ..brul::WindowConfig::default()
    }
}

pub fn windows() -> Vec<brul::WindowDef> {
    vec![
        brul::WindowDef { name: "Styled", config: window(), view },
    ]
}

pub fn script() -> Option<brul::script::Program> {
    None
}

pub fn names() -> &'static [&'static str] {
    &[]
}
"#;
    assert_eq!(emit::module(source).expect("module"), expected);
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
        module.contains("brul::BindingTarget::Class { table: class_none, expression: "),
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
        module.contains("styled(brul::gui::Widget::text(\"hi\"), &[\"a\"], theme)?"),
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
        r#"handlers.push(brul::Handler::new(component, Some(widget1.id()), brul::HandlerEvent::Click, "toggle"));"#
    ));
    assert!(module.contains(
        r#"handlers.push(brul::Handler::new(component, Some(widget1.id()), brul::HandlerEvent::Enter, "toggle"));"#
    ));
    assert!(module.contains("brul::gui::Widget::column(vec!["));
    assert!(module.contains(".draggable(true);"));
    assert!(!module.contains("HandlerEvent::Drag"));
}

#[test]
fn emits_if_structure() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        if (shown) {
            Text { "a" }
        } else {
            Text { "b" }
        }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "fn structure0_value(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {"
    ));
    assert!(module.contains(
        "fn structure0_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {\n    brul::gui::style::check_theme(styles(), theme).map_err(|error| error.to_string())?;"
    ));
    assert!(module.contains("brul::Value::Bool(true) => vec!["));
    assert!(module.contains("brul::Value::Bool(false) => vec!["));
    assert!(module.contains(
        "brul::register_structure(component, widget1.id(), false, structure0_value, structure0_rebuild);"
    ));
    assert!(module.contains(r#""if condition expects Bool, got {}""#));
}

#[test]
fn emits_each_structure() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Column {
        each (items as item, position (item.name)) { Text { item.name } }
    }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains(
        "fn structure0_value(component: &mut brul::Component) -> Result<brul::Value, brul::Error> {"
    ));
    assert!(module.contains(
        "fn structure0_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {\n    brul::gui::style::check_theme(styles(), theme).map_err(|error| error.to_string())?;"
    ));
    assert!(module.contains("component.borrow_mut().push_build_scope(vec!["));
    assert!(module.contains(
        "brul::Variable { name: brul::VariableId::from_index(1), value: item.clone() },"
    ));
    assert!(module.contains(
        r#"brul::Variable { name: brul::VariableId::from_index(2), value: brul::Value::Number(position as f64) },"#
    ));
    assert!(module.contains("let key = structure0_key(&mut component.borrow_mut()).ok();"));
    assert!(module.contains("rows.push(brul::RowInfo { key, item, start, count });"));
    assert!(module.contains(
        "brul::register_structure(component, widget1.id(), true, structure0_value, structure0_rebuild);"
    ));
}

#[test]
fn emits_window_handler() {
    let source = r#"<script>
    function quit() { }
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
        "pub fn window() -> brul::WindowConfig {\n    brul::WindowConfig {\n        title: \"demo\".to_owned(),\n        width: 488,\n        height: 800,\n        frameless: true,\n        transparent: true,\n        ..brul::WindowConfig::default()\n    }\n}"
    ));
}

#[test]
fn emits_every_window_with_its_flags() {
    let source = r#"<markup>
window Picker (title: "bropicker", width: 488, height: 800, frameless: true, transparent: true) {
    View { }
}

window Settings (title: "settings", width: 760, height: 560, frameless: false) {
    Text { "settings" }
}
</markup>
"#;
    let module = emit::module(source).expect("module");

    assert!(module.contains("pub fn view1(theme: Option<&str>"));
    assert!(module.contains(
        "pub fn window1() -> brul::WindowConfig {\n    brul::WindowConfig {\n        title: \"settings\".to_owned(),\n        width: 760,\n        height: 560,\n        frameless: false,\n        transparent: false,"
    ));
    assert!(module.contains("name: \"Picker\", config: window(), view }"));
    assert!(module.contains("name: \"Settings\", config: window1(), view: view1 }"));
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
</script>

<markup>
window Demo (title: "demo") {
    View { }
}
</markup>
"#;
    let module = emit::module(source).expect("module");
    assert!(module.contains("Some(brul::script::Program {"));
    assert!(module.contains(
        r#"brul::script::Statement::Let { constant: true, name: "limit".to_owned(), value: Some("#
    ));
    assert!(module.contains(
        r#"brul::script::Function { name: "bump".to_owned(), params: vec!["step".to_owned()], body: vec![brul::script::Statement::If {"#
    ));
    assert!(module.contains("brul::script::Statement::Block(vec![brul::script::Statement::Expr("));
    assert!(module.contains("brul::ast::ExprKind::Assign {"));
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
    assert!(module.contains("pub fn component0_view(theme: Option<&str>"));
    assert!(module.contains("pub fn component0_script() -> Option<brul::script::Program>"));
    assert!(module.contains("let built = brul::instance(component, brul::Instance {"));
    assert!(module.contains("bindings.extend(built.bindings);"));
    assert!(module.contains("props.extend(built.props);"));
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
    assert!(badge.source.contains("pub fn component0_view"));
    assert!(!badge.source.contains("pub fn view("));
    assert!(module.root.contains("pub mod bc_badge {"));
    assert!(module.root.contains("pub use self::bc_badge::*;"));
    assert!(module.root.contains("pub fn view("));
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
                "<script>\n    function launch() {\n    }\n</script>\n\n<use>\n    import Badge from \"./badge.brul\";\n</use>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    Column {\n        Badge (on clicked: launch) { }\n    }\n}\n</markup>\n",
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
        each (browsers as browser, index (browser.name)) {
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
            "s::Value::Gradient(s::Gradient { angle: 90.0, from: s::GradientColor::Color(s::Color::rgba(255, 0, 0, 255)), to: s::GradientColor::Color(s::Color::rgba(0, 0, 255, 255)) })"
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
        module.contains("pub fn component0_resource_check() -> &'static [u8]"),
        "{module}"
    );
    assert!(
        module.contains("brul::gui::Image::from_bytes(component0_resource_check())"),
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

    let plain = "<style>\n.panel { radius: 4px; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (panel) { }\n}\n</markup>\n";
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

    let plain = "<style>\n.list { radius: 4px; }\n</style>\n\n<markup>\nwindow Demo (title: \"demo\") {\n    View (list) { }\n}\n</markup>\n";
    let module = emit::module(plain).unwrap();
    assert!(!module.contains("s::Kind::Scroll"), "{module}");
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

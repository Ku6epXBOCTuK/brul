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
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    let mut props = Vec::new();
    let widget = match theme {
        None => {
            {
                let widget0 = brul_gui::Widget::row(vec![
                    {
                        let widget1 = brul_gui::Widget::text("a");
                        widget1
                    },
                    {
                        let widget2 = brul_gui::Widget::column(vec![]);
                        widget2
                    },
                ]);
                widget0
            }
        }
        Some(theme) => return Err(format!("unknown theme '{theme}'")),
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        structures,
        props,
    })
}

pub fn window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "demo".to_owned(),
        width: 960,
        height: 640,
        ..brul::WindowConfig::default()
    }
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
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut structures = Vec::new();
    let mut props = Vec::new();
    let widget = match theme {
        None => {
            {
                let widget0 = brul_gui::Widget::column(vec![])
                .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10), ..brul_gui::LayoutStyle::default() })
                .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(108, 140, 255, 255), ..brul_gui::WidgetStyle::default() }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255), ..brul_gui::WidgetStyle::default() }));
                widget0
            }
        }
        Some("dark") => {
            {
                let widget0 = brul_gui::Widget::column(vec![])
                .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10), ..brul_gui::LayoutStyle::default() })
                .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(108, 140, 255, 255), ..brul_gui::WidgetStyle::default() }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255), ..brul_gui::WidgetStyle::default() }));
                widget0
            }
        }
        Some("light") => {
            {
                let widget0 = brul_gui::Widget::column(vec![])
                .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10), ..brul_gui::LayoutStyle::default() })
                .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(17, 34, 51, 255), ..brul_gui::WidgetStyle::default() }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255), ..brul_gui::WidgetStyle::default() }));
                widget0
            }
        }
        Some(theme) => return Err(format!("unknown theme '{theme}'")),
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        structures,
        props,
    })
}

pub fn window() -> brul::WindowConfig {
    brul::WindowConfig {
        title: "styled".to_owned(),
        width: 960,
        height: 640,
        ..brul::WindowConfig::default()
    }
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
    assert!(module.contains(r#"let widget0 = brul_gui::Widget::text("");"#));
    assert!(module.contains(
        r#"bindings.push(brul::Binding::new(component, widget0.id(), brul::BindingTarget::Text(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read("name")?; Ok(value) })));"#
    ));
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
    assert!(
        module.contains("vertical_align: brul_gui::VerticalAlign::Center"),
        "{module}"
    );
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
        module.contains(
            r#"brul_gui::Widget::text_with_styles("hi", brul_gui::TextStyleSet::new(brul_gui::TextStyle { weight: 500, ..brul_gui::TextStyle::default() }).with(brul_gui::StyleState::Hovered, brul_gui::TextStyle { weight: 600, ..brul_gui::TextStyle::default() }))"#
        ),
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
        r#"brul::BindingTarget::Offset { x: Some(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read("left")?; Ok(value) }"#
    ));
    assert!(
        module.contains(
            r#"y: Some(|component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = component.read("top")?; Ok(value) }"#
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
    assert!(module.contains("brul_gui::Widget::column(vec!["));
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
        "fn structure0_rebuild(theme: Option<&str>, component: &brul::ComponentHandle) -> brul::Rebuilt {"
    ));
    assert!(
        module.contains(
            "fn structure0_rebuild_0(component: &brul::ComponentHandle) -> brul::Rebuilt {"
        )
    );
    assert!(module.contains("brul::Value::Bool(true) => vec!["));
    assert!(module.contains("brul::Value::Bool(false) => vec!["));
    assert!(module.contains(
        "structures.push(brul::Structure { parent: widget1.id(), each: false, value: structure0_value, rebuild: structure0_rebuild });"
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
    assert!(
        module.contains(
            "fn structure0_rebuild_0(component: &brul::ComponentHandle) -> brul::Rebuilt {"
        )
    );
    assert!(module.contains("component.borrow_mut().push_build_scope(&["));
    assert!(module.contains(r#"brul::Variable { name: "item", value: item.clone() },"#));
    assert!(module.contains(
        r#"brul::Variable { name: "position", value: brul::Value::Number(position as f64) },"#
    ));
    assert!(module.contains("let key = structure0_key(&mut component.borrow_mut()).ok();"));
    assert!(module.contains("rows.push(brul::RowInfo { key, item, start, count });"));
    assert!(module.contains("each: true, value: structure0_value"));
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
        "pub fn window() -> brul::WindowConfig {\n    brul::WindowConfig {\n        title: \"demo\".to_owned(),\n        width: 488,\n        height: 800,\n        ..brul::WindowConfig::default()\n    }\n}"
    ));
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
    emit::operation(&brul_syntax::parser::parse_expression(source).expect("expression parses"))
}

#[test]
fn emits_value_literals_and_reads() {
    assert_eq!(operated("x"), "component.read(\"x\")?");
    assert_eq!(operated("42"), "brul::Value::Number(42.0)");
    assert_eq!(operated("\"hi\""), "brul::Value::String(\"hi\".to_owned())");
    assert_eq!(operated("true"), "brul::Value::Bool(true)");
    assert_eq!(operated("null"), "brul::Value::Null");
    assert_eq!(
        operated("a.b"),
        "brul::eval::read_field(&(component.read(\"a\")?), \"b\")?"
    );
    assert_eq!(
        operated("a[0]"),
        "brul::eval::read_index(&(component.read(\"a\")?), &(brul::Value::Number(0.0)))?"
    );
}

#[test]
fn emits_value_operations() {
    assert_eq!(
        operated("!x"),
        "brul::eval::apply_unary(brul::ast::UnaryOp::Not, component.read(\"x\")?)?"
    );
    assert_eq!(
        operated("1 + 2"),
        "brul::eval::apply_binary(brul::ast::BinaryOp::Add, brul::Value::Number(1.0), brul::Value::Number(2.0))?"
    );
    assert_eq!(
        operated("[1, x]"),
        "brul::Value::Array(vec![brul::Value::Number(1.0), component.read(\"x\")?])"
    );
    assert_eq!(
        operated("{ a: 1 }"),
        "brul::Value::Object(vec![(\"a\".to_owned(), brul::Value::Number(1.0))])"
    );
    assert_eq!(
        operated("`v=${x}`"),
        "brul::Value::String(brul::eval::template(vec![brul::Value::String(\"v=\".to_owned()), component.read(\"x\")?])?)"
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
}

#[test]
fn emits_value_assignments() {
    let source = operated("a.b = 1");
    assert!(
        source.contains("let value = brul::Value::Number(1.0);"),
        "{source}"
    );
    assert!(
        source.contains("let mut container0 = component.read(\"a\")?;"),
        "{source}"
    );
    assert!(
        source.contains("brul::eval::assign_field(&mut container0, \"b\", value.clone())?;"),
        "{source}"
    );
    assert!(
        source.contains("component.assign(\"a\", container0.clone())?;"),
        "{source}"
    );
    assert!(source.ends_with("value }"), "{source}");

    assert!(
        operated("items[0] = x").contains("brul::eval::assign_index("),
        "{}",
        operated("items[0] = x")
    );
    assert!(
        operated("x += 1").contains("brul::ast::BinaryOp::Add, component.read(\"x\")?"),
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
        "brul::PropArgument { name: \"label\", ty: brul::PropType::String, expression: |component: &mut brul::Component| -> Result<brul::Value, brul::Error> { let value = brul::Value::String(\"hi\".to_owned()); Ok(value) }"
    ));
    assert!(module.contains("brul::PropArgument { name: \"size\", ty: brul::PropType::Number,"));
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

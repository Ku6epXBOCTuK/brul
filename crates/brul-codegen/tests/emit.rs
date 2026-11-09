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
    let expected = r#"pub fn view() -> brul_gui::Widget {
    brul_gui::Widget::row(vec![
        brul_gui::Widget::text("a"),
        brul_gui::Widget::column(vec![]),
    ])
}
"#;
    assert_eq!(emit::module(source).expect("module"), expected);
}

#[test]
fn emits_resolved_classes() {
    let source = r#"<style>
.box {
    width: 10px;
    padding: 2px;
    background: #102030;
    radius: 4px;
}
.label {
    color: #ff0000;
    font-size: 20px;
}
</style>

<markup>
window Styled (title: "styled") {
    View (box) {
        Text (label) { "hi" }
    }
}
</markup>
"#;
    let expected = r#"pub fn view() -> brul_gui::Widget {
    brul_gui::Widget::column(vec![
        brul_gui::Widget::styled_text("hi", brul_gui::TextStyle { font_size: 20.0f32, line_height: 24.0f32 })
        .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { text: brul_gui::Color::rgba(255, 0, 0, 255) })),
    ])
    .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10), padding: brul_gui::Padding { left: 2, top: 2, right: 2, bottom: 2 } })
    .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(16, 32, 48, 255), radius: 4 }))
}
"#;
    assert_eq!(emit::module(source).expect("module"), expected);
}

#[test]
fn rejects_non_static_content() {
    let source = r#"<markup>
window Demo (title: "demo") {
    Text { name }
}
</markup>
"#;
    let error = emit::module(source).expect_err("error");
    assert_eq!(
        error.message,
        "generated Text supports only string literals yet"
    );
}

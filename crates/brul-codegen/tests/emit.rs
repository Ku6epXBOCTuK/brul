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
    let expected = r#"pub fn view(theme: Option<&str>) -> Result<brul_gui::Widget, String> {
    match theme {
        None => Ok(
            brul_gui::Widget::row(vec![
                brul_gui::Widget::text("a"),
                brul_gui::Widget::column(vec![]),
            ]),
        ),
        Some(theme) => Err(format!("unknown theme '{theme}'")),
    }
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
    let expected = r#"pub fn view(theme: Option<&str>) -> Result<brul_gui::Widget, String> {
    match theme {
        None => Ok(
            brul_gui::Widget::column(vec![])
            .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10) })
            .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(108, 140, 255, 255) }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255) })),
        ),
        Some("dark") => Ok(
            brul_gui::Widget::column(vec![])
            .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10) })
            .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(108, 140, 255, 255) }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255) })),
        ),
        Some("light") => Ok(
            brul_gui::Widget::column(vec![])
            .with_layout(brul_gui::LayoutStyle { width: brul_gui::Dimension::Px(10) })
            .with_styles(brul_gui::StyleSet::new(brul_gui::WidgetStyle { background: brul_gui::Color::rgba(17, 34, 51, 255) }).with(brul_gui::StyleState::Checked, brul_gui::WidgetStyle { background: brul_gui::Color::rgba(255, 0, 0, 255) })),
        ),
        Some(theme) => Err(format!("unknown theme '{theme}'")),
    }
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

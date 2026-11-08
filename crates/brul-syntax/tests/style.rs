use brul_syntax::style::{
    Border, ClassRule, Color, Gradient, Property, State, Stylesheet, Theme, Value, parse_file,
};

fn sheet(rules: Vec<ClassRule>) -> Stylesheet {
    Stylesheet {
        themes: Vec::new(),
        rules,
    }
}

fn rule(class: &str, properties: Vec<Property>) -> ClassRule {
    ClassRule {
        class: class.to_owned(),
        state: None,
        properties,
    }
}

fn state_rule(class: &str, state: State) -> ClassRule {
    ClassRule {
        class: class.to_owned(),
        state: Some(state),
        properties: Vec::new(),
    }
}

fn property(name: &str, value: Value) -> Property {
    Property {
        name: name.to_owned(),
        value,
    }
}

#[test]
fn empty_stylesheet_parses() {
    assert_eq!(parse_file("").unwrap(), sheet(Vec::new()));
    assert_eq!(
        parse_file("\n  // only a comment\n\t").unwrap(),
        sheet(Vec::new())
    );
}

#[test]
fn empty_class_rules_parse_in_order() {
    let source = "\
.check-row { }
.checkbox { }
.text-main1 { }
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule("check-row", Vec::new()),
            rule("checkbox", Vec::new()),
            rule("text-main1", Vec::new()),
        ])
    );
}

#[test]
fn comments_between_rules_are_ignored() {
    let source = "// base\n.a { } // trailing\n// between\n.b { }\n";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("a", Vec::new()), rule("b", Vec::new())])
    );
}

#[test]
fn properties_parse_with_values_and_order() {
    let source = r#".item {
        width: 100%;
        padding: 4px;
        opacity: 0.5;
        font-family: "Segoe UI";
        justify-content: space-between;
        border-width: 2px;
        background: transparent;
    }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "item",
            vec![
                property("width", Value::Percent(100.0)),
                property("padding", Value::Px(4.0)),
                property("opacity", Value::Number(0.5)),
                property("font-family", Value::String("Segoe UI".to_owned())),
                property(
                    "justify-content",
                    Value::Keyword("space-between".to_owned())
                ),
                property("border-width", Value::Px(2.0)),
                property("background", Value::Color(Color::TRANSPARENT)),
            ],
        )])
    );
}

#[test]
fn comments_and_blank_lines_inside_a_rule_are_ignored() {
    let source = ".a {\n    // geometry\n    width: 12px;\n\n    height: 8px;\n}\n";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("width", Value::Px(12.0)),
                property("height", Value::Px(8.0)),
            ],
        )])
    );
}

#[test]
fn colors_parse_hex_and_keywords() {
    let source = r#".a {
        background: #3b82f6;
        border-color: #fff;
        color: #1e293b4d;
        tint: white;
        background: transparent;
    }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("background", Value::Color(Color::rgb(0x3b, 0x82, 0xf6))),
                property("border-color", Value::Color(Color::rgb(0xff, 0xff, 0xff))),
                property("color", Value::Color(Color::rgba(0x1e, 0x29, 0x3b, 0x4d))),
                property("tint", Value::Color(Color::WHITE)),
                property("background", Value::Color(Color::TRANSPARENT)),
            ],
        )])
    );
}

#[test]
fn themes_and_refs_parse() {
    let source = r#"
theme light {
    accent: #3b82f6;
    radius: 12px;
}
theme dark {
    accent: #3b82f6;
    radius: 12px;
}
.a { background: theme.accent; color: theme.text-main; }
"#;
    let accent = Value::Color(Color::rgb(0x3b, 0x82, 0xf6));
    assert_eq!(
        parse_file(source).unwrap(),
        Stylesheet {
            themes: vec![
                Theme {
                    name: "light".to_owned(),
                    properties: vec![
                        property("accent", accent.clone()),
                        property("radius", Value::Px(12.0)),
                    ],
                },
                Theme {
                    name: "dark".to_owned(),
                    properties: vec![
                        property("accent", accent),
                        property("radius", Value::Px(12.0)),
                    ],
                },
            ],
            rules: vec![rule(
                "a",
                vec![
                    property("background", Value::ThemeRef("accent".to_owned())),
                    property("color", Value::ThemeRef("text-main".to_owned())),
                ],
            )],
        }
    );
}

#[test]
fn gradients_parse() {
    let source = r#".app-icon {
        background: linear-gradient(135deg, #3b82f6, #06b6d4);
        border-color: linear-gradient(90, white, transparent);
    }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "app-icon",
            vec![
                property(
                    "background",
                    Value::Gradient(Gradient {
                        angle: 135.0,
                        from: Color::rgb(0x3b, 0x82, 0xf6),
                        to: Color::rgb(0x06, 0xb6, 0xd4),
                    }),
                ),
                property(
                    "border-color",
                    Value::Gradient(Gradient {
                        angle: 90.0,
                        from: Color::WHITE,
                        to: Color::TRANSPARENT,
                    }),
                ),
            ],
        )])
    );
}

#[test]
fn border_shortcut_parses() {
    let source = r#".box {
        border: 2px #3b82f6;
        border-bottom: 1.5px white;
    }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "box",
            vec![
                property(
                    "border",
                    Value::Border(Border {
                        width: 2.0,
                        color: Color::rgb(0x3b, 0x82, 0xf6),
                    }),
                ),
                property(
                    "border-bottom",
                    Value::Border(Border {
                        width: 1.5,
                        color: Color::WHITE,
                    }),
                ),
            ],
        )])
    );
}

#[test]
fn class_states_parse() {
    let source = "\
.a:hover { }
.b:pressed { }
.c:checked { }
.d:selected { }
.e:disabled { }
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            state_rule("a", State::Hover),
            state_rule("b", State::Pressed),
            state_rule("c", State::Checked),
            state_rule("d", State::Selected),
            state_rule("e", State::Disabled),
        ])
    );
}

#[test]
fn errors_point_at_the_source() {
    let sources = [
        ("a { }", "expected '.'", 1, 1),
        (". { }", "expected name", 1, 3),
        (".a", "expected '{'", 1, 3),
        (".a {", "expected '}'", 1, 5),
        (".a { }\n.b {", "expected '}'", 2, 5),
        (".a { width 4px; }", "expected ':'", 1, 12),
        (".a { width: 4px }", "expected ';'", 1, 17),
        (".a { width: ; }", "expected value", 1, 13),
        (".a { width: 4em; }", "expected ';'", 1, 14),
        (".a { width: \"x; }", "unterminated string", 1, 13),
        (".a { color: #12; }", "expected 3, 6 or 8 hex digits", 1, 13),
        (
            ".a { color: #12345; }",
            "expected 3, 6 or 8 hex digits",
            1,
            13,
        ),
        (
            ".a { color: #1234567; }",
            "expected 3, 6 or 8 hex digits",
            1,
            13,
        ),
        (
            ".a { color: #ggg; }",
            "expected 3, 6 or 8 hex digits",
            1,
            13,
        ),
        ("theme { }", "expected name", 1, 7),
        ("theme light { accent #fff; }", "expected ':'", 1, 22),
        ("theme light { accent: #fff }", "expected ';'", 1, 28),
        (".a { color: theme; }", "expected '.'", 1, 18),
        (".a { color: theme.accent }", "expected ';'", 1, 26),
        (
            ".a { b: linear-gradient(1deg #fff, #000); }",
            "expected ','",
            1,
            30,
        ),
        (
            ".a { b: linear-gradient(1deg, #fff); }",
            "expected ','",
            1,
            35,
        ),
        (
            ".a { b: linear-gradient(1deg, theme.x, #000); }",
            "expected color",
            1,
            31,
        ),
        (
            ".a { b: linear-gradient(deg, #fff, #000); }",
            "expected number",
            1,
            25,
        ),
        (".a:hovered { }", "unknown state", 1, 4),
        (".a: { }", "expected name", 1, 5),
        (".a { b: 4px theme.x; }", "expected color", 1, 13),
        (".a { b: 4px 5px; }", "expected ';'", 1, 13),
    ];
    for (source, message, line, column) in sources {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message.contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message
        );
        assert_eq!(
            (error.line, error.column),
            (line, column),
            "{source}: unexpected position"
        );
    }
}

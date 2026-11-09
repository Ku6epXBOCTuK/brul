use brul_syntax::style::{
    Border, BorderColor, ClassRule, Color, Gradient, Property, State, Stylesheet, Theme, Value,
    parse_file,
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
    text-main: #0f172a;
}
theme dark {
    accent: #3b82f6;
    radius: 12px;
    text-main: #0f172a;
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
                        property("text-main", Value::Color(Color::rgb(0x0f, 0x17, 0x2a))),
                    ],
                },
                Theme {
                    name: "dark".to_owned(),
                    properties: vec![
                        property("accent", accent),
                        property("radius", Value::Px(12.0)),
                        property("text-main", Value::Color(Color::rgb(0x0f, 0x17, 0x2a))),
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
fn theme_keys_take_keyword_colors() {
    let source = "theme light { accent: white; shade: transparent; }";
    assert_eq!(
        parse_file(source).unwrap().themes[0].properties,
        vec![
            property("accent", Value::Color(Color::WHITE)),
            property("shade", Value::Color(Color::TRANSPARENT)),
        ]
    );
}

#[test]
fn the_first_theme_is_the_default() {
    let source =
        "theme dark { accent: #000; }\ntheme light { accent: #fff; }\n.a { color: theme.accent; }";
    let sheet = parse_file(source).unwrap();
    assert_eq!(sheet.themes[0].name, "dark");
    assert_eq!(sheet.themes[1].name, "light");
}

#[test]
fn theme_keys_may_come_in_any_order() {
    let source =
        "theme light { accent: #fff; radius: 4px; }\ntheme dark { radius: 8px; accent: #000; }";
    assert!(parse_file(source).is_ok());
}

#[test]
fn gradients_parse() {
    let source = r#"
    .app-icon {
        background: linear-gradient(135deg, #3b82f6, #06b6d4);
    }
    .panel {
        background: linear-gradient(90, white, transparent);
    }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule(
                "app-icon",
                vec![property(
                    "background",
                    Value::Gradient(Gradient {
                        angle: 135.0,
                        from: Color::rgb(0x3b, 0x82, 0xf6),
                        to: Color::rgb(0x06, 0xb6, 0xd4),
                    }),
                )],
            ),
            rule(
                "panel",
                vec![property(
                    "background",
                    Value::Gradient(Gradient {
                        angle: 90.0,
                        from: Color::WHITE,
                        to: Color::TRANSPARENT,
                    }),
                )],
            ),
        ])
    );
}

#[test]
fn border_shortcut_parses() {
    let source = r#".box { border: 2px #3b82f6; }
.small { border: 1.5px white; }
.themed { border: 2px theme.border-main; }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule(
                "box",
                vec![property(
                    "border",
                    Value::Border(Border {
                        width: 2.0,
                        color: BorderColor::Color(Color::rgb(0x3b, 0x82, 0xf6)),
                    }),
                )],
            ),
            rule(
                "small",
                vec![property(
                    "border",
                    Value::Border(Border {
                        width: 1.5,
                        color: BorderColor::Color(Color::WHITE),
                    }),
                )],
            ),
            rule(
                "themed",
                vec![property(
                    "border",
                    Value::Border(Border {
                        width: 2.0,
                        color: BorderColor::ThemeRef("border-main".to_owned()),
                    }),
                )],
            ),
        ])
    );
}

#[test]
fn class_states_parse() {
    let source = "\
.a { }
.a:hover { }
.b { }
.b:pressed { }
.c { }
.c:checked { }
.d { }
.d:selected { }
.e { }
.e:disabled { }
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule("a", Vec::new()),
            state_rule("a", State::Hover),
            rule("b", Vec::new()),
            state_rule("b", State::Pressed),
            rule("c", Vec::new()),
            state_rule("c", State::Checked),
            rule("d", Vec::new()),
            state_rule("d", State::Selected),
            rule("e", Vec::new()),
            state_rule("e", State::Disabled),
        ])
    );
}

#[test]
fn mvp_properties_accept_their_types() {
    let source = r#".a {
        stretch: 2;
        align-items: stretch;
        font-weight: 700;
        opacity: 1;
        background: theme.accent;
        text-align: center;
    }"#;
    assert!(parse_file(source).is_ok());
}

#[test]
fn states_allow_only_visual_properties() {
    let source = "\
.badge { background: #fff; }
.badge:hover { background: #000; border-color: theme.accent; opacity: 0.5; }
.text { width: 40px; height: 20px; }
.text:hover { font-weight: 700; }
";
    assert!(parse_file(source).is_ok());
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
        (".a { width: ; }", "expected px or % for 'width'", 1, 13),
        (".a { width: 4em; }", "expected px or % for 'width'", 1, 14),
        (".a { font-family: \"x; }", "unterminated string", 1, 19),
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
            ".a { background: linear-gradient(1deg #fff, #000); }",
            "expected ','",
            1,
            39,
        ),
        (
            ".a { background: linear-gradient(1deg, #fff); }",
            "expected ','",
            1,
            44,
        ),
        (
            ".a { background: linear-gradient(1deg, theme.x, #000); }",
            "expected color",
            1,
            40,
        ),
        (
            ".a { background: linear-gradient(deg, #fff, #000); }",
            "expected number",
            1,
            34,
        ),
        (
            ".a { width: 4px; } .a:hover { width: 8px; }",
            "property 'width' cannot change in a state",
            1,
            31,
        ),
        (
            ".a { width: 4px; } .a:hover { font-size: 12px; }",
            "property 'font-size' cannot change in a state",
            1,
            31,
        ),
        (
            ".a:hover { font-weight: 700; }",
            "state '.a:hover' needs a base '.a'",
            1,
            2,
        ),
        (
            ".a { width: 8px; } .a:hover { font-weight: 700; }",
            "font-weight in a state needs width and height in the base class",
            1,
            31,
        ),
        (".a { widht: 4px; }", "unknown property 'widht'", 1, 6),
        (".a { width: red; }", "expected px or % for 'width'", 1, 13),
        (
            ".a { opacity: 2; }",
            "expected number from 0 to 1 for 'opacity'",
            1,
            15,
        ),
        (
            ".a { font-weight: 650; }",
            "expected one of 400, 500, 600, 700 for 'font-weight'",
            1,
            19,
        ),
        (
            ".a { justify-content: middle; }",
            "expected 'start', 'center', 'end', 'space-between' for 'justify-content'",
            1,
            23,
        ),
        (".a:hovered { }", "unknown state", 1, 4),
        (".a: { }", "expected name", 1, 5),
        (".a { border: 4px red; }", "expected color", 1, 18),
        (".a { border: 4px theme; }", "expected '.'", 1, 23),
        (
            ".a { border: 4px 5px; }",
            "expected color for 'border'",
            1,
            18,
        ),
        (
            "theme light { accent: #fff; }\ntheme dark { accent: #000; radius: 4px; }",
            "theme 'dark' has unknown key 'radius'",
            2,
            28,
        ),
        (
            "theme light { accent: #fff; radius: 4px; }\ntheme dark { accent: #000; }",
            "theme 'dark' is missing key 'radius'",
            2,
            7,
        ),
        (
            "theme light { accent: #fff; }\ntheme dark { accent: 12px; }",
            "theme 'dark' has a different type for 'accent'",
            2,
            14,
        ),
        (
            "theme light { accent: #fff; }\ntheme light { accent: #fff; }",
            "duplicate theme 'light'",
            2,
            7,
        ),
        (
            "theme light { accent: 4; }",
            "expected color or px for theme key 'accent'",
            1,
            23,
        ),
        (
            "theme light { accent: red; }",
            "expected color or px for theme key 'accent'",
            1,
            23,
        ),
        (
            "theme light { accent: \"#fff\"; }",
            "expected color or px for theme key 'accent'",
            1,
            23,
        ),
        (
            "theme light { accent: #fff; accent: #000; }",
            "duplicate theme key 'accent'",
            1,
            29,
        ),
        (
            "theme light { accent: #fff; }\n.a { width: theme.accent; }",
            "theme key 'accent' is a color, but 'width' expects a px length",
            2,
            6,
        ),
        (
            "theme light { radius: 4px; }\n.a { background: theme.radius; }",
            "theme key 'radius' is a px length, but 'background' expects a color",
            2,
            6,
        ),
        (
            "theme light { accent: #fff; }\n.a { color: theme.muted; }",
            "unknown theme key 'muted'",
            2,
            6,
        ),
        (
            ".a { stretch: theme.x; }",
            "expected number for 'stretch'",
            1,
            15,
        ),
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

use brul_syntax::style::{ClassRule, Property, Stylesheet, Value, parse_file};

fn rule(class: &str, properties: Vec<Property>) -> ClassRule {
    ClassRule {
        class: class.to_owned(),
        properties,
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
    assert_eq!(parse_file("").unwrap(), Stylesheet { rules: Vec::new() });
    assert_eq!(
        parse_file("\n  // only a comment\n\t").unwrap(),
        Stylesheet { rules: Vec::new() }
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
        Stylesheet {
            rules: vec![
                rule("check-row", Vec::new()),
                rule("checkbox", Vec::new()),
                rule("text-main1", Vec::new()),
            ],
        }
    );
}

#[test]
fn comments_between_rules_are_ignored() {
    let source = "// base\n.a { } // trailing\n// between\n.b { }\n";
    assert_eq!(
        parse_file(source).unwrap(),
        Stylesheet {
            rules: vec![rule("a", Vec::new()), rule("b", Vec::new())],
        }
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
        Stylesheet {
            rules: vec![rule(
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
                    property("background", Value::Keyword("transparent".to_owned())),
                ],
            )],
        }
    );
}

#[test]
fn comments_and_blank_lines_inside_a_rule_are_ignored() {
    let source = ".a {\n    // geometry\n    width: 12px;\n\n    height: 8px;\n}\n";
    assert_eq!(
        parse_file(source).unwrap(),
        Stylesheet {
            rules: vec![rule(
                "a",
                vec![
                    property("width", Value::Px(12.0)),
                    property("height", Value::Px(8.0)),
                ],
            )],
        }
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

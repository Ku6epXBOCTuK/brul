use brul_syntax::style::{ClassRule, Stylesheet, parse_file};

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
                ClassRule {
                    class: "check-row".to_owned(),
                },
                ClassRule {
                    class: "checkbox".to_owned(),
                },
                ClassRule {
                    class: "text-main1".to_owned(),
                },
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
            rules: vec![
                ClassRule {
                    class: "a".to_owned(),
                },
                ClassRule {
                    class: "b".to_owned(),
                },
            ],
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
        (".a { width: 1px; }", "expected '}'", 1, 6),
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

use super::*;
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
            ".a { background: linear-gradient(1deg, rose, #000); }",
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
            ".a { clip: 1; }",
            "expected true or false for 'clip'",
            1,
            12,
        ),
        (
            ".a { scroll: 1; }",
            "expected true or false for 'scroll'",
            1,
            14,
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
            "theme light { radius: 4px; }\n.a { border: 2px theme.radius; }",
            "theme key 'radius' is a px length, but 'border-top-color' expects a color",
            2,
            6,
        ),
        (
            ".a { flex-grow: theme.x; }",
            "expected number for 'flex-grow'",
            1,
            17,
        ),
    ];
    for (source, message, line, column) in sources {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
        assert_eq!(
            (error.line(), error.column()),
            (line, column),
            "{source}: unexpected position"
        );
    }
}

#[test]
fn an_override_parses_into_rules_for_a_component() {
    let source = "\
theme dark { surface: #1e293b; }
.card { background: #000000; }
override Card {
    .card { background: theme.surface; }
    .card:hover { }
}
";
    let parsed = parse_root(source).unwrap();
    assert_eq!(parsed.stylesheet.rules.len(), 1);
    assert_eq!(parsed.overrides.len(), 1);
    let over = &parsed.overrides[0];
    assert_eq!(over.component, "Card");
    assert_eq!(
        over.rules,
        vec![
            rule(
                "card",
                vec![property(
                    "background",
                    Value::ThemeRef("surface".to_owned())
                )]
            ),
            state_rule("card", State::Hover),
        ]
    );
}

#[test]
fn an_override_is_rejected_outside_the_root_stylesheet() {
    for error in [
        parse_file("override Card { .card { } }").unwrap_err(),
        parse_file_in("override Card { .card { } }", &[]).unwrap_err(),
    ] {
        assert_eq!(
            error.message(),
            "an override can only be declared in the root <style>"
        );
        assert_eq!((error.line(), error.column()), (1, 9));
    }
}

#[test]
fn an_override_checks_its_theme_refs() {
    let source = "\
theme dark { surface: #1e293b; }
override Card { .card { background: theme.muted; } }
";
    let error = parse_root(source).unwrap_err();
    assert_eq!(error.message(), "unknown theme key 'muted'");
    assert_eq!(error.line(), 2);
}

#[test]
fn animations_parse_with_their_durations() {
    let source =
        ".a { background: #ffffff; animate background: 300ms; animate box-shadow-color: 150ms; }";
    let parsed = parse_file(source).unwrap();
    assert_eq!(
        parsed.rules[0].animations,
        vec![
            Animation {
                property: Kind::Background,
                duration: 300.0,
            },
            Animation {
                property: Kind::ShadowColor,
                duration: 150.0,
            },
        ]
    );
}

#[test]
fn wrong_animations_are_rejected() {
    let sources = [
        (".a { animate widht: 300ms; }", "unknown property 'widht'"),
        (
            ".a { animate width: 300ms; }",
            "property 'width' cannot be animated",
        ),
        (
            ".a { animate background: 300px; }",
            "expected ms for 'animate background'",
        ),
        (
            ".a { animate background: 300s; }",
            "expected ms for 'animate background'",
        ),
        (
            ".a { } .a:hover { animate background: 300ms; }",
            "animate cannot be used in a state",
        ),
    ];
    for (source, message) in sources {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
    }
}

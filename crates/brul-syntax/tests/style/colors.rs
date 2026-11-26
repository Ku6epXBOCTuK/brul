use super::*;
#[test]
fn color_expressions_parse() {
    let source = r#".a {
    background: mix(theme.accent, white, 20%);
    border-color: alpha(#3b82f6, 40%);
    box-shadow-color: lighten(#0f172a, 12%);
}
.b { background: darken(lighten(theme.surface-main, 20%), 5%); }"#;
    let sheet = parse_file(source).expect("sheet");
    let value = |rule: usize, kind: Kind| {
        sheet.rules[rule]
            .properties
            .iter()
            .find(|property| property.kind == kind)
            .map(|property| &property.value)
            .expect("property")
    };

    assert_eq!(
        value(0, Kind::Background),
        &Value::ColorExpr(ColorExpr::Mix {
            from: ColorValue::Theme("accent".to_string()),
            to: ColorValue::Color(Color::WHITE),
            amount: 20.0,
        })
    );
    assert_eq!(
        value(0, Kind::BorderColorSide(Side::Top)),
        &Value::ColorExpr(ColorExpr::Alpha {
            color: ColorValue::Color(Color::rgb(0x3b, 0x82, 0xf6)),
            amount: 40.0,
        })
    );
    assert_eq!(
        value(0, Kind::ShadowColor),
        &Value::ColorExpr(ColorExpr::Lighten {
            color: ColorValue::Color(Color::rgb(0x0f, 0x17, 0x2a)),
            amount: 12.0,
        })
    );
    let nested = value(1, Kind::Background);
    let Value::ColorExpr(ColorExpr::Darken { color, amount }) = nested else {
        panic!("expected darken, got {nested:?}");
    };
    assert_eq!(*amount, 5.0);
    assert!(matches!(
        color,
        ColorValue::Expr(inner) if matches!(**inner, ColorExpr::Lighten { .. })
    ));
}

#[test]
fn color_expression_errors_name_the_problem() {
    let deep = format!(
        ".a {{ background: {}#3b82f6{}; }}",
        "alpha(".repeat(9),
        ", 10%)".repeat(9)
    );
    let sources = [
        (
            ".a { background: alpha(#3b82f6, 50); }".to_string(),
            "expected percent from 0 to 100 for 'background'",
        ),
        (
            ".a { background: lighten(#3b82f6, 101%); }".to_string(),
            "expected percent from 0 to 100 for 'background'",
        ),
        (
            ".a { background: mix(#3b82f6 20%); }".to_string(),
            "expected ','",
        ),
        (deep, "color expression is nested deeper than 8"),
        (
            "theme light { radius: 4px; }\n.a { background: alpha(theme.radius, 50%); }"
                .to_string(),
            "theme key 'radius' is a px length, but 'background' expects a color",
        ),
        (
            "theme light { accent: #111111; }\n.a { background: darken(theme.muted, 5%); }"
                .to_string(),
            "unknown theme key 'muted'",
        ),
        (
            ".a { grid-template-columns: red 1fr; }".to_string(),
            "expected a track size for 'grid-template-columns'",
        ),
        (
            ".a { grid-row: 1 / nope; }".to_string(),
            "expected a line placement for 'grid-row'",
        ),
    ];
    for (source, message) in sources {
        let error = parse_file(&source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
    }
}

#[test]
fn grid_track_lists_parse() {
    let source = "\
.grid {
    display: grid;
    grid-template-columns: 100px 1fr minmax(80px, 2fr);
    grid-template-rows: 50% 2fr;
}
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "grid",
            vec![
                property("display", Value::Keyword("grid".to_owned())),
                property(
                    "grid-template-columns",
                    Value::TrackList(vec![
                        TrackSize::Px(100.0),
                        TrackSize::Fr(1.0),
                        TrackSize::Minmax(
                            Box::new(TrackSize::Px(80.0)),
                            Box::new(TrackSize::Fr(2.0))
                        ),
                    ])
                ),
                property(
                    "grid-template-rows",
                    Value::TrackList(vec![TrackSize::Percent(50.0), TrackSize::Fr(2.0)])
                ),
            ]
        )])
    );
}

#[test]
fn grid_repeat_tracks_parse() {
    let source = "\
.grid {
    grid-template-columns: repeat(2, 100px 1fr) repeat(auto-fit, 40px);
}
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "grid",
            vec![property(
                "grid-template-columns",
                Value::TrackList(vec![
                    TrackSize::Repeat(
                        RepeatCount::Count(2),
                        vec![TrackSize::Px(100.0), TrackSize::Fr(1.0)]
                    ),
                    TrackSize::Repeat(RepeatCount::AutoFit, vec![TrackSize::Px(40.0)]),
                ])
            )]
        )])
    );
}

#[test]
fn grid_lines_parse() {
    let source = "\
.item {
    grid-column: 1 / span 2;
    grid-row: -1 / auto;
    grid-auto-flow: column;
}
";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "item",
            vec![
                property(
                    "grid-column",
                    Value::GridLine(GridLine {
                        start: LinePlacement::Line(1),
                        end: Some(LinePlacement::Span(2)),
                    })
                ),
                property(
                    "grid-row",
                    Value::GridLine(GridLine {
                        start: LinePlacement::Line(-1),
                        end: Some(LinePlacement::Auto),
                    })
                ),
                property("grid-auto-flow", Value::Keyword("column".to_owned())),
            ]
        )])
    );
}

#[test]
fn grid_repeat_accepts_auto_fill() {
    let source = ".grid { grid-template-columns: repeat(auto-fill, 10px); }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "grid",
            vec![property(
                "grid-template-columns",
                Value::TrackList(vec![TrackSize::Repeat(
                    RepeatCount::AutoFill,
                    vec![TrackSize::Px(10.0)]
                )])
            )]
        )])
    );
}

#[test]
fn grid_errors_point_inside_the_value() {
    let cases = [
        (
            ".grid { grid-template-columns: 100px minmax(80px, bogus); }",
            "bogus",
            "expected a track size for 'grid-template-columns'",
        ),
        (
            ".grid { grid-template-columns: repeat(0, 1fr); }",
            "0,",
            "expected number from 1 to 65535 for 'grid-template-columns'",
        ),
        (
            ".grid { grid-row: 2 / nope; }",
            "nope",
            "expected a line placement for 'grid-row'",
        ),
        (
            ".grid { grid-template-columns: 1fr 2fr,; }",
            ",;",
            "expected ';'",
        ),
    ];
    for (source, needle, message) in cases {
        let error = parse_file(source).unwrap_err();
        assert!(
            error.message().contains(message),
            "{source}: expected message containing {message}, got {}",
            error.message()
        );
        assert_eq!(error.line(), 1, "{source}");
        assert_eq!(
            error.column(),
            source.find(needle).unwrap() + 1,
            "{source}: unexpected column"
        );
    }
}

#[test]
fn deeply_nested_grid_tracks_are_rejected() {
    let source = format!(
        ".grid {{ grid-template-columns: {}1fr{}; }}",
        "repeat(2, ".repeat(9),
        ")".repeat(9)
    );
    let error = parse_file(&source).unwrap_err();
    assert!(
        error
            .message()
            .contains("grid track is nested deeper than 8"),
        "{}",
        error.message()
    );
}

#[test]
fn grid_areas_parse() {
    let source = "\
.grid {
    grid-template-areas: \"head head\" \"side main\";
}
.grid-item {
    grid-area: main;
}
";
    let rows = ["head head".to_owned(), "side main".to_owned()];
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule(
                "grid",
                vec![property(
                    "grid-template-areas",
                    Value::AreaTemplate(AreaTemplate::parse(&rows).unwrap())
                )]
            ),
            rule(
                "grid-item",
                vec![property("grid-area", Value::GridArea("main".to_owned()))]
            )
        ])
    );
}

#[test]
fn malformed_grid_areas_are_rejected() {
    let source = ".grid { grid-template-areas: \"a b\" \"b a\"; }";
    assert!(parse_file(source).is_err());
}

#[test]
fn bindings_are_declared_next_to_their_rules() {
    let source = "@apply TextInput brul-input;\n@apply on drag brul-draggable;\n.brul-input { cursor: text; }\n.brul-draggable { cursor: grab; }";
    let builtin = parse_builtin(source, &[]).unwrap();
    assert_eq!(builtin.bindings.len(), 2);
    assert_eq!(
        builtin.bindings[0].target,
        Target::Element("TextInput".into())
    );
    assert_eq!(builtin.bindings[0].class, "brul-input");
    assert_eq!(builtin.bindings[1].target, Target::Event("drag".into()));
    assert_eq!(builtin.bindings[1].class, "brul-draggable");
    assert_eq!(builtin.stylesheet.rules.len(), 2);
}

#[test]
fn builtin_rules_use_the_root_palette() {
    let palette = parse_file("theme dark { accent: #fff; }").unwrap().themes;
    let source = "@apply TextInput brul-input;\n.brul-input { color: theme.accent; }";
    let builtin = parse_builtin(source, &palette).unwrap();
    assert_eq!(builtin.stylesheet.themes, palette);
}

#[test]
fn a_binding_needs_a_declared_class() {
    let source = "@apply TextInput brul-missing;";
    let error = parse_builtin(source, &[]).unwrap_err();
    assert_eq!(
        error.message(),
        "binding references unknown class 'brul-missing'"
    );
}

#[test]
fn a_binding_target_cannot_repeat() {
    let source = "@apply TextInput brul-input;\n@apply TextInput brul-other;\n.brul-input { }\n.brul-other { }";
    let error = parse_builtin(source, &[]).unwrap_err();
    assert!(error.message().contains("duplicate binding"));
}

#[test]
fn a_reserved_class_cannot_be_declared_by_the_user() {
    let error = parse_file(".brul-input { cursor: text; }").unwrap_err();
    assert_eq!(error.message(), "class name 'brul-input' is reserved");
}

#[test]
fn bindings_are_only_allowed_in_the_builtin_stylesheet() {
    let error = parse_file("@apply TextInput brul-input;").unwrap_err();
    assert_eq!(
        error.message(),
        "bindings are only allowed in the builtin stylesheet"
    );
}

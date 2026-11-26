use super::*;
#[test]
fn theme_keys_take_keyword_colors() {
    let source = "theme light { accent: white; shade: transparent; }";
    assert_eq!(
        parse_file(source).unwrap().themes[0].properties,
        vec![
            entry("accent", Value::Color(Color::WHITE)),
            entry("shade", Value::Color(Color::TRANSPARENT)),
        ]
    );
}

#[test]
fn theme_keys_take_color_expressions() {
    let source = "theme light { accent: #3b82f6; ring: alpha(theme.accent, 40%); shade: darken(white, 6%); }";
    assert_eq!(
        parse_file(source).unwrap().themes[0].properties,
        vec![
            entry("accent", Value::Color(Color::rgb(0x3b, 0x82, 0xf6))),
            entry(
                "ring",
                Value::ColorExpr(ColorExpr::Alpha {
                    color: ColorValue::Theme("accent".to_string()),
                    amount: 40.0,
                })
            ),
            entry(
                "shade",
                Value::ColorExpr(ColorExpr::Darken {
                    color: ColorValue::Color(Color::WHITE),
                    amount: 6.0,
                })
            ),
        ]
    );
}

#[test]
fn theme_key_errors_name_the_problem() {
    let sources = [
        (
            "theme light { accent: #3b82f6; ring: alpha(theme.muted, 40%); }",
            "theme 'light' has unknown key 'muted'",
        ),
        (
            "theme light { accent: #3b82f6; ring: alpha(theme.ring, 40%); }",
            "theme 'light' has a cycle: ring -> ring",
        ),
        (
            "theme light {
    accent: #3b82f6;
    a: alpha(theme.b, 10%);
    b: alpha(theme.a, 10%);
}",
            "theme 'light' has a cycle: a -> b -> a",
        ),
        (
            "theme light { radius: 4px; shade: alpha(theme.radius, 10%); }",
            "theme key 'radius' is a px length, but 'shade' needs a color",
        ),
        (
            "theme light { accent: #fff; copy: theme.accent; }",
            "theme key 'copy' cannot alias another key",
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

#[test]
fn the_first_theme_is_the_default() {
    let source =
        "theme dark { accent: #000; }\ntheme light { accent: #fff; }\n.a { color: theme.accent; }";
    let sheet = parse_file(source).unwrap();
    assert_eq!(sheet.themes[0].name, "dark");
    assert_eq!(sheet.themes[1].name, "light");
}

#[test]
fn an_imported_stylesheet_takes_the_palette_it_is_given() {
    let palette = parse_file("theme dark { accent: #fff; }\ntheme light { accent: #000; }")
        .unwrap()
        .themes;
    let sheet = parse_file_in(".a { color: theme.accent; }", &palette).expect("sheet");
    assert_eq!(sheet.themes, palette);
    assert_eq!(sheet.rules.len(), 1);
}

#[test]
fn an_imported_stylesheet_cannot_declare_a_theme() {
    let palette = parse_file("theme dark { accent: #fff; }").unwrap().themes;
    let error = parse_file_in("theme dark { accent: #000; }", &palette).unwrap_err();
    assert_eq!(
        error.message(),
        "a theme can only be declared in the root <style>"
    );
    assert_eq!((error.line(), error.column()), (1, 6));
}

#[test]
fn an_imported_stylesheet_checks_refs_against_the_palette() {
    let palette = parse_file("theme dark { accent: #fff; }").unwrap().themes;
    let error = parse_file_in(".a { color: theme.muted; }", &palette).unwrap_err();
    assert_eq!(error.message(), "unknown theme key 'muted'");
    assert_eq!(error.line(), 1);
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
                        from: ColorValue::Color(Color::rgb(0x3b, 0x82, 0xf6)),
                        to: ColorValue::Color(Color::rgb(0x06, 0xb6, 0xd4)),
                    }),
                )],
            ),
            rule(
                "panel",
                vec![property(
                    "background",
                    Value::Gradient(Gradient {
                        angle: 90.0,
                        from: ColorValue::Color(Color::WHITE),
                        to: ColorValue::Color(Color::TRANSPARENT),
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
    let mut box_properties = border_width(Value::Px(2.0));
    box_properties.extend(border_color(Value::Color(Color::rgb(0x3b, 0x82, 0xf6))));
    let mut small_properties = border_width(Value::Px(1.5));
    small_properties.extend(border_color(Value::Color(Color::WHITE)));
    let mut themed_properties = border_width(Value::Px(2.0));
    themed_properties.extend(border_color(Value::ThemeRef("border-main".to_owned())));
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule("box", box_properties),
            rule("small", small_properties),
            rule("themed", themed_properties),
        ])
    );
}

#[test]
fn box_shadow_shortcut_parses_offsets_blur_and_color() {
    let source = r#".a { box-shadow: 0px 10px 15px #3b82f640; }
.negative { box-shadow: -4px 2px 0px transparent; }
.themed { box-shadow: 1px 2px 3px theme.shadow; }"#;
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![
            rule(
                "a",
                vec![
                    property("box-shadow-offset-x", Value::Px(0.0)),
                    property("box-shadow-offset-y", Value::Px(10.0)),
                    property("box-shadow-blur", Value::Px(15.0)),
                    property(
                        "box-shadow-color",
                        Value::Color(Color::rgba(0x3b, 0x82, 0xf6, 0x40)),
                    ),
                ],
            ),
            rule(
                "negative",
                vec![
                    property("box-shadow-offset-x", Value::Px(-4.0)),
                    property("box-shadow-offset-y", Value::Px(2.0)),
                    property("box-shadow-blur", Value::Px(0.0)),
                    property("box-shadow-color", Value::Color(Color::TRANSPARENT)),
                ],
            ),
            rule(
                "themed",
                vec![
                    property("box-shadow-offset-x", Value::Px(1.0)),
                    property("box-shadow-offset-y", Value::Px(2.0)),
                    property("box-shadow-blur", Value::Px(3.0)),
                    property("box-shadow-color", Value::ThemeRef("shadow".to_owned())),
                ],
            ),
        ])
    );
}

#[test]
fn box_shadow_geometry_in_a_state_is_rejected() {
    let error = parse_file(
        ".a { box-shadow: 0px 10px 15px transparent; }\n.a:hover { box-shadow: 4px 4px 4px #fff; }",
    )
    .expect_err("box-shadow in a state");

    assert_eq!(
        error.message(),
        "property 'box-shadow-offset-x' cannot change in a state"
    );
}

#[test]
fn box_shadow_color_in_a_state_is_allowed() {
    let source =
        ".a { box-shadow: 0px 10px 15px transparent; }\n.a:hover { box-shadow-color: #3b82f640; }";
    assert!(parse_file(source).is_ok());
}

#[test]
fn box_shadow_needs_a_color_and_a_non_negative_blur() {
    let missing_color = parse_file(".a { box-shadow: 0px 10px 15px; }").unwrap_err();
    assert_eq!(missing_color.message(), "expected color for 'box-shadow'");

    let negative_blur = parse_file(".a { box-shadow: 0px 10px -3px #fff; }").unwrap_err();
    assert_eq!(negative_blur.message(), "expected px for 'box-shadow'");
}

#[test]
fn border_in_a_state_is_rejected_through_its_width() {
    let error = parse_file(".a { border: 2px #3b82f6; }\n.a:hover { border: 4px #3b82f6; }")
        .expect_err("border in a state");

    assert_eq!(
        error.message(),
        "property 'border-top-width' cannot change in a state"
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
.f { }
.f:focus { }
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
            rule("f", Vec::new()),
            state_rule("f", State::Focus),
        ])
    );
}

#[test]
fn mvp_properties_accept_their_types() {
    let source = r#".a {
        flex-grow: 2;
        flex-shrink: 0;
        flex-basis: 0px;
        display: flex;
        flex-direction: row;
        align-items: stretch;
        font-weight: 700;
        opacity: 1;
        background: theme.accent;
        text-align: center;
    }"#;
    assert!(parse_file(source).is_ok());
}

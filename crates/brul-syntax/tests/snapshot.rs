use brul_syntax::style::{
    Animation, ClassRule, Color, ColorValue, Gradient, Kind, Property, SnapshotError, State,
    Stylesheet, Theme, ThemeEntry, Value, parse_file,
};

fn gray(value: u8) -> Value {
    Value::Color(Color::rgb(value, value, value))
}

#[test]
fn classes_merge_left_to_right() {
    let sheet = parse_file(".a { width: 4px; color: #111111; }\n.b { width: 8px; padding: 2px; }")
        .expect("sheet");
    let snapshot = sheet.snapshot(&["a", "b"], None, None).expect("snapshot");

    assert_eq!(snapshot.get(Kind::Width), Some(&Value::Px(8.0)));
    assert_eq!(snapshot.get(Kind::Color), Some(&gray(0x11)));
    assert_eq!(
        snapshot.get(Kind::PaddingSide(brul_syntax::style::Side::Left)),
        Some(&Value::Px(2.0))
    );
}

#[test]
fn property_order_follows_first_definition() {
    let sheet =
        parse_file(".a { width: 4px; color: #111111; }\n.b { width: 8px; }").expect("sheet");
    let snapshot = sheet.snapshot(&["a", "b"], None, None).expect("snapshot");
    let names: Vec<&str> = snapshot
        .properties()
        .iter()
        .map(|property| property.kind.name())
        .collect();

    assert_eq!(names, ["width", "color"]);
}

#[test]
fn state_overrides_are_applied_on_top_of_the_base() {
    let sheet = parse_file(
        ".a { width: 4px; color: #111111; }\n.a:hover { color: #222222; }\n.b { color: #333333; }",
    )
    .expect("sheet");
    let snapshot = sheet
        .snapshot(&["a", "b"], Some(State::Hover), None)
        .expect("snapshot");

    assert_eq!(snapshot.get(Kind::Color), Some(&gray(0x22)));
    assert_eq!(snapshot.get(Kind::Width), Some(&Value::Px(4.0)));
}

#[test]
fn inactive_states_are_ignored() {
    let sheet =
        parse_file(".a { color: #111111; }\n.a:checked { color: #222222; }").expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], Some(State::Hover), None)
        .expect("snapshot");

    assert_eq!(snapshot.get(Kind::Color), Some(&gray(0x11)));
}

#[test]
fn theme_refs_resolve_to_the_selected_theme() {
    let sheet = parse_file(
        "theme light { accent: #111111; }\ntheme dark { accent: #222222; }\n.a { color: theme.accent; }",
    )
    .expect("sheet");
    let light = sheet.snapshot(&["a"], None, Some("light")).expect("light");
    let dark = sheet.snapshot(&["a"], None, Some("dark")).expect("dark");

    assert_eq!(light.get(Kind::Color), Some(&gray(0x11)));
    assert_eq!(dark.get(Kind::Color), Some(&gray(0x22)));
}

#[test]
fn default_theme_is_the_first_one() {
    let sheet = parse_file(
        "theme dark { accent: #222222; }\ntheme light { accent: #111111; }\n.a { color: theme.accent; }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert_eq!(snapshot.get(Kind::Color), Some(&gray(0x22)));
}

#[test]
fn unknown_theme_and_key_are_errors() {
    let sheet =
        parse_file("theme light { accent: #111111; }\n.a { color: theme.accent; }").expect("sheet");
    assert_eq!(
        sheet.snapshot(&["a"], None, Some("dark")),
        Err(SnapshotError::UnknownTheme("dark".to_string()))
    );

    let sheet = Stylesheet {
        themes: vec![Theme {
            name: "light".to_string(),
            properties: vec![ThemeEntry {
                key: "accent".to_string(),
                value: gray(0x11),
            }],
        }],
        rules: vec![ClassRule {
            class: "a".to_string(),
            state: None,
            properties: vec![Property {
                kind: Kind::Color,
                value: Value::ThemeRef("muted".to_string()),
            }],
            animations: Vec::new(),
        }],
    };
    assert_eq!(
        sheet.snapshot(&["a"], None, None),
        Err(SnapshotError::UnknownThemeKey("muted".to_string()))
    );
}

#[test]
fn gradient_stops_resolve_to_the_selected_theme() {
    let sheet = parse_file(
        "theme light { from: #111111; to: #222222; }\ntheme dark { from: #333333; to: #444444; }\n.a { background: linear-gradient(135deg, theme.from, theme.to); }",
    )
    .expect("sheet");
    let light = sheet.snapshot(&["a"], None, Some("light")).expect("light");
    let dark = sheet.snapshot(&["a"], None, Some("dark")).expect("dark");

    let expected = |from: u8, to: u8| {
        Value::Gradient(Gradient {
            angle: 135.0,
            from: ColorValue::Color(Color::rgb(from, from, from)),
            to: ColorValue::Color(Color::rgb(to, to, to)),
        })
    };

    assert_eq!(light.get(Kind::Background), Some(&expected(0x11, 0x22)));
    assert_eq!(dark.get(Kind::Background), Some(&expected(0x33, 0x44)));
}

#[test]
fn a_gradient_stop_with_a_bad_theme_key_is_rejected() {
    let unknown = parse_file(
        "theme light { from: #111111; }\n.a { background: linear-gradient(90, theme.accent, #222222); }",
    )
    .unwrap_err();
    assert_eq!(unknown.message, "unknown theme key 'accent'");

    let not_a_color = parse_file(
        "theme light { radius: 4px; }\n.a { background: linear-gradient(90, theme.radius, #222222); }",
    )
    .unwrap_err();
    assert_eq!(
        not_a_color.message,
        "theme key 'radius' is a px length, but 'background' expects a color"
    );
}

#[test]
fn border_theme_refs_resolve_to_a_color() {
    let sheet = parse_file(
        "theme light { border-main: #111111; radius: 4px; }\n.a { border: 2px theme.border-main; }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert_eq!(snapshot.get(Kind::BorderWidth), Some(&Value::Px(2.0)));
    assert_eq!(
        snapshot.get(Kind::BorderColor),
        Some(&Value::Color(Color::rgb(0x11, 0x11, 0x11)))
    );
}

#[test]
fn border_theme_ref_to_a_non_color_is_rejected_at_parse_time() {
    let error =
        parse_file("theme light { radius: 4px; }\n.a { border: 2px theme.radius; }").unwrap_err();

    assert_eq!(
        error.message,
        "theme key 'radius' is a px length, but 'border-color' expects a color"
    );
}

#[test]
fn literals_survive_without_themes() {
    let sheet = parse_file(".a { border: 2px #111111; }").expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert!(matches!(
        snapshot.get(Kind::BorderColor),
        Some(Value::Color(_))
    ));
}

#[test]
fn snapshot_errors_render_distinct_messages() {
    let errors = [
        SnapshotError::UnknownTheme(String::from("dark")),
        SnapshotError::UnknownThemeKey(String::from("dark")),
    ];

    let mut messages: Vec<String> = errors.iter().map(|error| error.to_string()).collect();

    assert!(messages.iter().all(|message| message.contains("dark")));
    messages.sort();
    messages.dedup();
    assert_eq!(messages.len(), errors.len());
}

#[test]
fn animations_merge_with_the_rightmost_class_winning() {
    let sheet = parse_file(
        ".a { animate color: 100ms; animate opacity: 50ms; }\n.b { animate color: 300ms; }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a", "b"], None, None).expect("snapshot");

    assert_eq!(
        snapshot.animations().to_vec(),
        vec![
            Animation {
                property: Kind::Color,
                duration: 300.0,
            },
            Animation {
                property: Kind::Opacity,
                duration: 50.0,
            },
        ]
    );
}

#[test]
fn alpha_sets_only_the_alpha_channel() {
    let sheet = parse_file(
        "theme light { accent: #3b82f6; }\n.a { background: alpha(theme.accent, 50%); }",
    )
    .expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], None, Some("light"))
        .expect("snapshot");
    assert_eq!(
        snapshot.get(Kind::Background),
        Some(&Value::Color(Color::rgba(0x3b, 0x82, 0xf6, 128)))
    );
}

#[test]
fn mix_with_transparent_matches_alpha() {
    let sheet = parse_file(
        ".a { background: mix(#3b82f6, transparent, 50%); border-color: alpha(#3b82f6, 50%); }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");
    assert_eq!(
        snapshot.get(Kind::Background),
        snapshot.get(Kind::BorderColor)
    );
}

#[test]
fn mix_ends_on_its_operands() {
    let sheet = parse_file(
        "theme light { accent: #3b82f6; surface: #0f172a; }\n\
         .a { color: mix(theme.accent, theme.surface, 0%); border-color: mix(theme.accent, theme.surface, 100%); }",
    )
    .expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], None, Some("light"))
        .expect("snapshot");
    assert_eq!(
        snapshot.get(Kind::Color),
        Some(&Value::Color(Color::rgb(0x3b, 0x82, 0xf6)))
    );
    assert_eq!(
        snapshot.get(Kind::BorderColor),
        Some(&Value::Color(Color::rgb(0x0f, 0x17, 0x2a)))
    );
}

#[test]
fn lighten_and_darken_move_tone_by_absolute_points() {
    use material_colors::{color::Rgb, hct::Hct};

    let sheet = parse_file(
        "theme light { accent: #3b82f6; }\n\
         .a { background: lighten(theme.accent, 20%); border-color: darken(theme.accent, 10%); }",
    )
    .expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], None, Some("light"))
        .expect("snapshot");
    let tone = |kind: Kind| {
        let Some(Value::Color(color)) = snapshot.get(kind) else {
            panic!("expected a resolved color for {kind:?}");
        };
        Hct::new(Rgb::new(color.r, color.g, color.b)).get_tone()
    };
    let base = Hct::new(Rgb::new(0x3b, 0x82, 0xf6)).get_tone();
    assert!((tone(Kind::Background) - (base + 20.0)).abs() < 0.5);
    assert!((tone(Kind::BorderColor) - (base - 10.0)).abs() < 0.5);
}

#[test]
fn gradient_stops_take_color_expressions() {
    let sheet = parse_file(
        "theme light { accent: #3b82f6; }\n\
         .a { background: linear-gradient(90, alpha(theme.accent, 50%), lighten(theme.accent, 20%)); }",
    )
    .expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], None, Some("light"))
        .expect("snapshot");
    let Some(Value::Gradient(gradient)) = snapshot.get(Kind::Background) else {
        panic!("expected a gradient");
    };
    assert_eq!(
        gradient.from,
        ColorValue::Color(Color::rgba(0x3b, 0x82, 0xf6, 128))
    );
    let ColorValue::Color(to) = &gradient.to else {
        panic!("expected a resolved stop");
    };
    assert_ne!(*to, Color::rgb(0x3b, 0x82, 0xf6));
    assert_eq!(to.a, 255);
}

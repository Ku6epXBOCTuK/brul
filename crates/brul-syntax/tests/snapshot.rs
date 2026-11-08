use brul_syntax::style::{Border, BorderColor, Color, SnapshotError, State, Value, parse_file};

fn color(value: u8) -> Value {
    Value::Color(Color::rgb(value, value, value))
}

#[test]
fn classes_merge_left_to_right() {
    let sheet = parse_file(".a { width: 4px; color: #111111; }\n.b { width: 8px; padding: 2px; }")
        .expect("sheet");
    let snapshot = sheet.snapshot(&["a", "b"], None, None).expect("snapshot");

    assert_eq!(snapshot.get("width"), Some(&Value::Px(8.0)));
    assert_eq!(snapshot.get("color"), Some(&color(0x11)));
    assert_eq!(snapshot.get("padding"), Some(&Value::Px(2.0)));
}

#[test]
fn property_order_follows_first_definition() {
    let sheet =
        parse_file(".a { width: 4px; color: #111111; }\n.b { width: 8px; }").expect("sheet");
    let snapshot = sheet.snapshot(&["a", "b"], None, None).expect("snapshot");
    let names: Vec<&str> = snapshot
        .properties()
        .iter()
        .map(|property| property.name.as_str())
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

    assert_eq!(snapshot.get("color"), Some(&color(0x22)));
    assert_eq!(snapshot.get("width"), Some(&Value::Px(4.0)));
}

#[test]
fn inactive_states_are_ignored() {
    let sheet =
        parse_file(".a { color: #111111; }\n.a:checked { color: #222222; }").expect("sheet");
    let snapshot = sheet
        .snapshot(&["a"], Some(State::Hover), None)
        .expect("snapshot");

    assert_eq!(snapshot.get("color"), Some(&color(0x11)));
}

#[test]
fn theme_refs_resolve_to_the_selected_theme() {
    let sheet = parse_file(
        "theme light { accent: #111111; }\ntheme dark { accent: #222222; }\n.a { color: theme.accent; }",
    )
    .expect("sheet");
    let light = sheet.snapshot(&["a"], None, Some("light")).expect("light");
    let dark = sheet.snapshot(&["a"], None, Some("dark")).expect("dark");

    assert_eq!(light.get("color"), Some(&color(0x11)));
    assert_eq!(dark.get("color"), Some(&color(0x22)));
}

#[test]
fn default_theme_is_the_first_one() {
    let sheet = parse_file(
        "theme dark { accent: #222222; }\ntheme light { accent: #111111; }\n.a { color: theme.accent; }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert_eq!(snapshot.get("color"), Some(&color(0x22)));
}

#[test]
fn unknown_theme_and_key_are_errors() {
    let sheet =
        parse_file("theme light { accent: #111111; }\n.a { color: theme.accent; }").expect("sheet");
    assert_eq!(
        sheet.snapshot(&["a"], None, Some("dark")),
        Err(SnapshotError::UnknownTheme("dark".to_string()))
    );

    let sheet =
        parse_file("theme light { accent: #111111; }\n.a { color: theme.muted; }").expect("sheet");
    assert_eq!(
        sheet.snapshot(&["a"], None, None),
        Err(SnapshotError::UnknownThemeKey("muted".to_string()))
    );
}

#[test]
fn border_theme_refs_resolve_to_a_color() {
    let sheet = parse_file(
        "theme light { border-main: #111111; radius: 4px; }\n.a { border: 2px theme.border-main; }",
    )
    .expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert_eq!(
        snapshot.get("border"),
        Some(&Value::Border(Border {
            width: 2.0,
            color: BorderColor::Color(Color::rgb(0x11, 0x11, 0x11)),
        }))
    );
}

#[test]
fn border_theme_ref_to_a_non_color_is_an_error() {
    let sheet = parse_file(
        "theme light { border-main: #111111; radius: 4px; }\n.a { border: 2px theme.radius; }",
    )
    .expect("sheet");

    assert_eq!(
        sheet.snapshot(&["a"], None, None),
        Err(SnapshotError::NotAColor("radius".to_string()))
    );
}

#[test]
fn literals_survive_without_themes() {
    let sheet = parse_file(".a { border: 2px #111111; }").expect("sheet");
    let snapshot = sheet.snapshot(&["a"], None, None).expect("snapshot");

    assert!(matches!(snapshot.get("border"), Some(Value::Border(_))));
}

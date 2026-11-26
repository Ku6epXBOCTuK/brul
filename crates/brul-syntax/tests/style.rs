#[path = "style/basic.rs"]
mod basic;
#[path = "style/colors.rs"]
mod colors;
#[path = "style/states.rs"]
mod states;

use brul_style::{AreaTemplate, GridLine, LinePlacement, RepeatCount, TrackSize};
use brul_syntax::style::{
    Animation, ClassRule, Color, ColorExpr, ColorValue, Gradient, Kind, Property, Side, State,
    Stylesheet, Target, Theme, ThemeEntry, Value, parse_builtin, parse_file, parse_file_in,
    parse_root,
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
        animations: Vec::new(),
    }
}

fn state_rule(class: &str, state: State) -> ClassRule {
    ClassRule {
        class: class.to_owned(),
        state: Some(state),
        properties: Vec::new(),
        animations: Vec::new(),
    }
}

fn property(name: &str, value: Value) -> Property {
    Property {
        kind: Kind::from_name(name).expect("known property"),
        value,
    }
}

fn border_width(value: Value) -> Vec<Property> {
    [
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
    ]
    .into_iter()
    .map(|name| property(name, value.clone()))
    .collect()
}

fn border_color(value: Value) -> Vec<Property> {
    [
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
    ]
    .into_iter()
    .map(|name| property(name, value.clone()))
    .collect()
}

fn entry(key: &str, value: Value) -> ThemeEntry {
    ThemeEntry {
        key: key.to_owned(),
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
fn clip_takes_a_bool() {
    let source = ".a { clip: true; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("a", vec![property("clip", Value::Bool(true))])])
    );
}

#[test]
fn scroll_takes_a_bool() {
    let source = ".a { scroll: true; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("a", vec![property("scroll", Value::Bool(true))])])
    );
}

#[test]
fn cursor_takes_a_keyword() {
    let source = ".a { cursor: pointer; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![property("cursor", Value::Keyword("pointer".to_owned()))]
        )])
    );
}

#[test]
fn cursor_can_change_in_a_visual_state() {
    let source = ".a { } .a:hover { cursor: not-allowed; }";
    let mut hovered = state_rule("a", State::Hover);
    hovered.properties = vec![property("cursor", Value::Keyword("not-allowed".to_owned()))];
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("a", Vec::new()), hovered])
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
    let mut properties = vec![
        property("width", Value::Percent(100.0)),
        property("padding-top", Value::Px(4.0)),
        property("padding-right", Value::Px(4.0)),
        property("padding-bottom", Value::Px(4.0)),
        property("padding-left", Value::Px(4.0)),
        property("opacity", Value::Number(0.5)),
        property("font-family", Value::String("Segoe UI".to_owned())),
        property(
            "justify-content",
            Value::Keyword("space-between".to_owned()),
        ),
    ];
    properties.extend(border_width(Value::Px(2.0)));
    properties.push(property("background", Value::Color(Color::TRANSPARENT)));
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("item", properties)])
    );
}

#[test]
fn padding_takes_one_to_four_values_in_css_order() {
    let source = ".a { padding: 1px 2px 3px; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("padding-top", Value::Px(1.0)),
                property("padding-right", Value::Px(2.0)),
                property("padding-bottom", Value::Px(3.0)),
                property("padding-left", Value::Px(2.0)),
            ],
        )])
    );
}

#[test]
fn padding_rejects_a_fifth_value() {
    let error = parse_file(".a { padding: 1px 2px 3px 4px 5px; }").unwrap_err();
    assert_eq!(error.message(), "expected at most 4 values for 'padding'");
}

#[test]
fn border_width_takes_one_to_four_values_in_css_order() {
    let source = ".a { border-width: 1px 2px 3px; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("border-top-width", Value::Px(1.0)),
                property("border-right-width", Value::Px(2.0)),
                property("border-bottom-width", Value::Px(3.0)),
                property("border-left-width", Value::Px(2.0)),
            ],
        )])
    );
}

#[test]
fn border_width_longhands_are_independent() {
    let source = ".a { border-left-width: 1px; border-right-width: 2px; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("border-left-width", Value::Px(1.0)),
                property("border-right-width", Value::Px(2.0)),
            ],
        )])
    );
}

#[test]
fn border_color_takes_one_to_four_values_in_css_order() {
    let source = ".a { border-color: #ff0000 #00ff00 #0000ff; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("border-top-color", Value::Color(Color::rgb(255, 0, 0))),
                property("border-right-color", Value::Color(Color::rgb(0, 255, 0))),
                property("border-bottom-color", Value::Color(Color::rgb(0, 0, 255))),
                property("border-left-color", Value::Color(Color::rgb(0, 255, 0))),
            ],
        )])
    );
}

#[test]
fn border_color_longhands_are_independent() {
    let source = ".a { border-left-color: #ff0000; border-right-color: #0000ff; }";
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule(
            "a",
            vec![
                property("border-left-color", Value::Color(Color::rgb(255, 0, 0))),
                property("border-right-color", Value::Color(Color::rgb(0, 0, 255))),
            ],
        )])
    );
}

#[test]
fn border_color_rejects_a_fifth_value() {
    let error =
        parse_file(".a { border-color: #ff0000 #00ff00 #0000ff #000000 #ffffff; }").unwrap_err();
    assert_eq!(
        error.message(),
        "expected at most 4 values for 'border-color'"
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
    let mut properties = vec![property(
        "background",
        Value::Color(Color::rgb(0x3b, 0x82, 0xf6)),
    )];
    properties.extend(border_color(Value::Color(Color::rgb(0xff, 0xff, 0xff))));
    properties.push(property(
        "color",
        Value::Color(Color::rgba(0x1e, 0x29, 0x3b, 0x4d)),
    ));
    properties.push(property("tint", Value::Color(Color::WHITE)));
    properties.push(property("background", Value::Color(Color::TRANSPARENT)));
    assert_eq!(
        parse_file(source).unwrap(),
        sheet(vec![rule("a", properties)])
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
                        entry("accent", accent.clone()),
                        entry("radius", Value::Px(12.0)),
                        entry("text-main", Value::Color(Color::rgb(0x0f, 0x17, 0x2a))),
                    ],
                },
                Theme {
                    name: "dark".to_owned(),
                    properties: vec![
                        entry("accent", accent),
                        entry("radius", Value::Px(12.0)),
                        entry("text-main", Value::Color(Color::rgb(0x0f, 0x17, 0x2a))),
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

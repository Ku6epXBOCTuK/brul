use brul_gui::style;
use brul_gui::{Dimension, Justify};
use brul_style::{ClassRule, Kind, Property, Stylesheet, Value};

fn property(kind: Kind, value: Value) -> Property {
    Property { kind, value }
}

fn rule(properties: Vec<Property>) -> Stylesheet {
    Stylesheet {
        themes: Vec::new(),
        rules: vec![ClassRule {
            class: "a".into(),
            state: None,
            properties,
            animations: Vec::new(),
        }],
    }
}

#[test]
fn a_value_is_read_by_the_type_of_its_spec() {
    let sheet = rule(vec![
        property(Kind::Width, Value::Percent(50.0)),
        property(Kind::Spacing, Value::Px(4.0)),
        property(Kind::FontFamily, Value::String("Inter".into())),
        property(Kind::FontWeight, Value::Number(600.0)),
        property(Kind::JustifyContent, Value::Keyword("space-between".into())),
        property(Kind::Clip, Value::Bool(true)),
    ]);
    let resolved = style::resolve(&sheet, &["a"], None).expect("a valid sheet resolves");

    assert_eq!(resolved.layout.width, Dimension::Percent(50.0));
    assert_eq!(resolved.layout.spacing, 4);
    assert_eq!(resolved.layout.justify, Justify::SpaceBetween);
    assert_eq!(resolved.text.base().family.as_deref(), Some("Inter"));
    assert_eq!(resolved.text.base().weight, 600);
    assert!(resolved.styles.base().clip);
}

#[test]
fn a_wrong_value_for_the_spec_type_is_reported_with_the_property_name() {
    let sheet = rule(vec![property(Kind::Spacing, Value::Number(4.0))]);
    let Err(error) = style::resolve(&sheet, &["a"], None) else {
        panic!("a number is not a px");
    };

    assert_eq!(error.to_string(), "expected px for 'spacing'");
}

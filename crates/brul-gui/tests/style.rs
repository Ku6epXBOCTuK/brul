use brul_gui::style;
use brul_gui::{Dimension, Justify, WidgetTree};
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
        property(Kind::Gap, Value::Px(4.0)),
        property(Kind::FontFamily, Value::String("Inter".into())),
        property(Kind::FontWeight, Value::Number(600.0)),
        property(Kind::JustifyContent, Value::Keyword("space-between".into())),
        property(Kind::Clip, Value::Bool(true)),
    ]);
    let resolved = style::resolve(&sheet, &["a"], None).expect("a valid sheet resolves");

    assert_eq!(resolved.layout.width, Dimension::Percent(50.0));
    assert_eq!(resolved.layout.gap, 4);
    assert_eq!(resolved.layout.justify, Justify::SpaceBetween);
    assert_eq!(resolved.text.base().family.as_deref(), Some("Inter"));
    assert_eq!(resolved.text.base().weight, 600);
    assert!(resolved.styles.base().clip);
}

#[test]
fn a_class_is_overlaid_on_the_styles_a_widget_already_has() {
    let sheet = Stylesheet {
        themes: Vec::new(),
        rules: vec![
            ClassRule {
                class: "base".into(),
                state: None,
                properties: vec![
                    property(Kind::Width, Value::Px(80.0)),
                    property(Kind::Height, Value::Px(20.0)),
                ],
                animations: Vec::new(),
            },
            ClassRule {
                class: "override".into(),
                state: None,
                properties: vec![property(Kind::Width, Value::Px(40.0))],
                animations: Vec::new(),
            },
        ],
    };
    let widget = style::styled(brul_gui::Widget::text("x"), &sheet, &["base"], None).expect("base");
    let id = widget.id();
    let widget = style::styled(widget, &sheet, &["override"], None).expect("override");
    let root = brul_gui::Widget::column(vec![widget]).with_layout(brul_gui::LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        ..Default::default()
    });
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut brul_gui::TextEngine::new(),
        brul_gui::Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("styled node");

    assert_eq!((node.width, node.height), (40, 20));
}

#[test]
fn a_class_carries_radius_shadow_scroll_and_text_geometry() {
    let sheet = rule(vec![
        property(Kind::BorderRadius, Value::Px(10.0)),
        property(Kind::Scroll, Value::Bool(true)),
        property(Kind::ShadowOffsetX, Value::Px(2.0)),
        property(Kind::ShadowOffsetY, Value::Px(-3.0)),
        property(Kind::ShadowBlur, Value::Px(4.0)),
        property(Kind::FontSize, Value::Px(18.0)),
        property(Kind::FontFamily, Value::String("Inter".into())),
    ]);
    let class = style::class(&sheet, "a", None).expect("class");
    assert_eq!(class.style.base().border_radius, Some(10));
    assert_eq!(class.style.base().scroll, Some(true));
    assert_eq!(class.style.base().shadow_x, Some(2));
    assert_eq!(class.style.base().shadow_y, Some(-3));
    assert_eq!(class.style.base().shadow_blur, Some(4));
    assert_eq!(class.text.base().size, Some(18.0));
    assert_eq!(class.text.base().family.as_deref(), Some("Inter"));

    let widget = style::styled(brul_gui::Widget::text("x"), &sheet, &["a"], None).expect("styled");
    let id = widget.id();
    let root = brul_gui::Widget::column(vec![widget]).with_layout(brul_gui::LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        ..Default::default()
    });
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut brul_gui::TextEngine::new(),
        brul_gui::Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("styled node");
    assert_eq!(node.style.border_radius, 10);
    assert!(node.style.scroll);
    assert_eq!(
        (
            node.style.shadow.x,
            node.style.shadow.y,
            node.style.shadow.blur
        ),
        (2, -3, 4)
    );
}

#[test]
fn a_wrong_value_for_the_spec_type_is_reported_with_the_property_name() {
    let sheet = rule(vec![property(Kind::Gap, Value::Number(4.0))]);
    let Err(error) = style::resolve(&sheet, &["a"], None) else {
        panic!("a number is not a px");
    };

    assert_eq!(error.to_string(), "expected px for 'gap'");
}

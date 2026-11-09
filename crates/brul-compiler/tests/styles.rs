use brul_compiler::build_tree;
use brul_compiler::styles::resolve;
use brul_gui::{
    Align, Color, Dimension, Justify, Padding, Size, TextAlign, TextEngine, WidgetState, WidgetTree,
};
use brul_syntax::markup::{Declaration, Member, parse_file};
use brul_syntax::style::{Stylesheet, parse_file as parse_style};

fn stylesheet(source: &str) -> Stylesheet {
    parse_style(source).expect("style parses")
}

fn state(
    hovered: bool,
    pressed: bool,
    checked: bool,
    selected: bool,
    disabled: bool,
) -> WidgetState {
    WidgetState {
        hovered,
        pressed,
        checked,
        selected,
        disabled,
        visible: true,
    }
}

#[test]
fn layout_and_widget_style_are_mapped() {
    let sheet = stylesheet(
        r#"
        .panel {
            width: 120px;
            height: 40%;
            padding: 6px;
            spacing: 8px;
            border: 2px #112233;
            radius: 5px;
            opacity: 0.5;
            background: #abcdef;
            color: #010203;
        }
        "#,
    );
    let resolved = resolve(&sheet, &["panel"], None).expect("styles resolve");
    let base = resolved.styles.base();

    assert_eq!(resolved.layout.width, Dimension::Px(120));
    assert_eq!(resolved.layout.height, Dimension::Percent(40.0));
    assert_eq!(resolved.layout.padding, Padding::all(6));
    assert_eq!(resolved.layout.spacing, 8);
    assert_eq!(resolved.layout.border_width, 2);
    assert_eq!(base.background, Color::rgb(0xab, 0xcd, 0xef));
    assert_eq!(base.border, Color::rgb(0x11, 0x22, 0x33));
    assert_eq!(base.text, Color::rgb(0x01, 0x02, 0x03));
    assert_eq!(base.radius, 5);
    assert_eq!(base.opacity, 0.5);
}

#[test]
fn padding_sides_override_the_shorthand() {
    let sheet = stylesheet(".panel { padding: 4px; padding-left: 10px; }");

    let resolved = resolve(&sheet, &["panel"], None).expect("styles resolve");

    assert_eq!(
        resolved.layout.padding,
        Padding {
            left: 10,
            top: 4,
            right: 4,
            bottom: 4,
        }
    );
}

#[test]
fn alignment_keywords_are_mapped() {
    let sheet = stylesheet(".panel { justify-content: space-between; align-items: stretch; }");

    let resolved = resolve(&sheet, &["panel"], None).expect("styles resolve");

    assert_eq!(resolved.layout.justify, Justify::SpaceBetween);
    assert_eq!(resolved.layout.align, Align::Stretch);
}

#[test]
fn text_style_is_mapped() {
    let sheet = stylesheet(
        r#".label { font-size: 24px; font-family: "Segoe UI"; font-weight: 700; text-align: center; }"#,
    );

    let resolved = resolve(&sheet, &["label"], None).expect("styles resolve");

    assert_eq!(resolved.text.font_size, 24.0);
    assert!((resolved.text.line_height - 28.8).abs() < 0.01);
    assert_eq!(resolved.text.family.as_deref(), Some("Segoe UI"));
    assert_eq!(resolved.text.weight, 700);
    assert_eq!(resolved.text.align, TextAlign::Center);
}

#[test]
fn states_override_the_base_style() {
    let sheet = stylesheet(
        r#"
        .item { width: 10px; height: 10px; background: #111111; }
        .item:hover { background: #222222; }
        .item:pressed { background: #333333; }
        .item:checked { background: #444444; }
        .item:selected { background: #555555; }
        .item:disabled { background: #666666; }
        "#,
    );
    let resolved = resolve(&sheet, &["item"], None).expect("styles resolve");

    assert_eq!(
        resolved
            .styles
            .resolve(state(false, false, false, false, false))
            .background,
        Color::rgb(0x11, 0x11, 0x11)
    );
    assert_eq!(
        resolved
            .styles
            .resolve(state(true, false, false, false, false))
            .background,
        Color::rgb(0x22, 0x22, 0x22)
    );
    assert_eq!(
        resolved
            .styles
            .resolve(state(false, true, false, false, false))
            .background,
        Color::rgb(0x33, 0x33, 0x33)
    );
    assert_eq!(
        resolved
            .styles
            .resolve(state(false, false, true, false, false))
            .background,
        Color::rgb(0x44, 0x44, 0x44)
    );
    assert_eq!(
        resolved
            .styles
            .resolve(state(false, false, false, true, false))
            .background,
        Color::rgb(0x55, 0x55, 0x55)
    );
    assert_eq!(
        resolved
            .styles
            .resolve(state(false, false, false, false, true))
            .background,
        Color::rgb(0x66, 0x66, 0x66)
    );
}

#[test]
fn theme_values_follow_the_selected_theme() {
    let sheet = stylesheet(
        r#"
        theme light { accent: #00ff00; }
        theme dark { accent: #ff0000; }
        .panel { background: theme.accent; }
        "#,
    );

    let light = resolve(&sheet, &["panel"], Some("light")).expect("light theme");
    let dark = resolve(&sheet, &["panel"], Some("dark")).expect("dark theme");

    assert_eq!(light.styles.base().background, Color::rgb(0x00, 0xff, 0x00));
    assert_eq!(dark.styles.base().background, Color::rgb(0xff, 0x00, 0x00));
}

#[test]
fn gradient_is_reported_as_unsupported() {
    let sheet = stylesheet(".panel { background: linear-gradient(90deg, #000000, #ffffff); }");

    let error = resolve(&sheet, &["panel"], None).err().expect("gradient");

    assert_eq!(error, "gradient is not supported yet");
}

#[test]
fn tree_applies_classes_to_the_root_widget() {
    let markup = r#"
        window Picker (title: "x") {
            Column (panel) {
                Text { "Hello" }
            }
        }
        "#;
    let file = parse_file(markup).expect("markup parses");
    let Declaration::Window(window) = &file.declarations[0] else {
        panic!("expected a window");
    };
    let element = window
        .members
        .iter()
        .find_map(|member| match member {
            Member::Element(element) => Some(element),
            _ => None,
        })
        .expect("window has a root element");
    let sheet =
        stylesheet(".panel { width: 200px; height: 100px; padding: 5px; background: #334455; }");
    let widget =
        build_tree(element, &sheet, None, &mut brul_script::Component::new()).expect("tree builds");
    let mut tree = WidgetTree::new(widget);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 300,
            height: 300,
        },
        1.0,
    )
    .expect("layout");
    let node = tree
        .snapshot()
        .expect("snapshot")
        .nodes
        .first()
        .cloned()
        .expect("root node");

    assert_eq!(node.width, 200);
    assert_eq!(node.height, 100);
    assert_eq!(node.style.background, Color::rgb(0x33, 0x44, 0x55));
    assert_eq!(node.content_x - node.x, 5);
    assert_eq!(node.content_y - node.y, 5);
}

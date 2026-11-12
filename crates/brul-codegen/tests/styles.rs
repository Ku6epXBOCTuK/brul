use std::time::Duration;

use brul_codegen::styles::resolve;
use brul_gui::{Align, Dimension, Gradient, Justify, Paint};

fn resolved(style: &str) -> brul_codegen::styles::Resolved {
    let stylesheet = brul_syntax::style::parse_file(style).expect("stylesheet");
    resolve(&stylesheet, &["a"], None).expect("resolved")
}

#[test]
fn size_takes_px_percent_and_a_stretch_factor() {
    let resolved = resolved(".a { width: 40px; height: 25%; stretch: 2; }");
    assert_eq!(resolved.layout.width, Dimension::Px(40));
    assert_eq!(resolved.layout.height, Dimension::Percent(25.0));
    assert_eq!(resolved.layout.stretch, 2.0);
}

#[test]
fn padding_covers_every_side_and_the_gap_between_children() {
    let all = resolved(".a { padding: 4px; }");
    assert_eq!(all.layout.padding, brul_gui::Padding::all(4));

    let sides = resolved(
        ".a { padding-left: 1px; padding-right: 2px; padding-top: 3px; padding-bottom: 4px; spacing: 5px; }",
    );
    assert_eq!(sides.layout.padding.left, 1);
    assert_eq!(sides.layout.padding.right, 2);
    assert_eq!(sides.layout.padding.top, 3);
    assert_eq!(sides.layout.padding.bottom, 4);
    assert_eq!(sides.layout.spacing, 5);
}

#[test]
fn border_sets_the_width_from_either_property() {
    let shorthand = resolved(".a { border: 3px #fff; }");
    assert_eq!(shorthand.layout.border_width, 3);

    let longhand = resolved(".a { border-width: 7px; }");
    assert_eq!(longhand.layout.border_width, 7);
}

#[test]
fn justify_content_maps_every_keyword() {
    let cases = [
        ("start", Justify::Start),
        ("center", Justify::Center),
        ("end", Justify::End),
        ("space-between", Justify::SpaceBetween),
    ];
    for (keyword, expected) in cases {
        let resolved = resolved(&format!(".a {{ justify-content: {keyword}; }}"));
        assert_eq!(resolved.layout.justify, expected, "{keyword}");
    }
}

#[test]
fn align_items_maps_every_keyword() {
    let cases = [
        ("start", Align::Start),
        ("center", Align::Center),
        ("end", Align::End),
        ("stretch", Align::Stretch),
    ];
    for (keyword, expected) in cases {
        let resolved = resolved(&format!(".a {{ align-items: {keyword}; }}"));
        assert_eq!(resolved.layout.align, expected, "{keyword}");
    }
}

#[test]
fn paint_covers_background_border_color_tint_and_the_gradient() {
    let resolved = resolved(
        ".a { background: #102030; border: 2px #405060; color: #708090; tint: #a0b0c0; radius: 6px; opacity: 0.5; }",
    );
    let style = resolved.styles.base();
    assert_eq!(
        style.background,
        Paint::Color(brul_gui::Color::rgb(0x10, 0x20, 0x30))
    );
    assert_eq!(style.border, brul_gui::Color::rgb(0x40, 0x50, 0x60));
    assert_eq!(style.text, brul_gui::Color::rgb(0x70, 0x80, 0x90));
    assert_eq!(style.tint, Some(brul_gui::Color::rgb(0xa0, 0xb0, 0xc0)));
    assert_eq!(style.radius, 6);
    assert_eq!(style.opacity, 0.5);
}

#[test]
fn box_shadow_sets_geometry_and_the_state_color() {
    let resolved = resolved(
        ".a { box-shadow: -2px 10px 15px transparent; }\n.a:hover { box-shadow-color: #3b82f640; }",
    );
    let base = resolved.styles.base().shadow;
    assert_eq!((base.x, base.y, base.blur), (-2, 10, 15));
    assert_eq!(base.color, brul_gui::Color::TRANSPARENT);

    let hovered = resolved
        .styles
        .resolve(brul_gui::StyleState::Hovered.flags())
        .shadow;
    assert_eq!(hovered.color, brul_gui::Color::rgba(0x3b, 0x82, 0xf6, 0x40));
    assert_eq!((hovered.x, hovered.y, hovered.blur), (-2, 10, 15));
}

#[test]
fn a_gradient_background_resolves_to_a_linear_paint() {
    let resolved = resolved(".a { background: linear-gradient(135deg, #fff, #000); }");

    assert_eq!(
        resolved.styles.base().background,
        Paint::Gradient(Gradient::Linear {
            angle: 135.0,
            from: brul_gui::Color::WHITE,
            to: brul_gui::Color::BLACK,
        })
    );
}

#[test]
fn font_covers_size_family_weight_and_alignment() {
    let resolved = resolved(
        ".a { font-size: 20px; font-family: \"Inter\"; font-weight: 600; text-align: center; }",
    );
    let text = resolved.text.base();
    assert_eq!(text.font_size, 20.0);
    assert_eq!(text.line_height, 24.0);
    assert_eq!(text.family.as_deref(), Some("Inter"));
    assert_eq!(text.weight, 600);
    assert_eq!(text.align, brul_gui::TextAlign::Center);
}

#[test]
fn font_weight_in_a_state_becomes_a_text_variant() {
    let resolved = resolved(
        ".a { width: 40px; height: 20px; font-weight: 500; } .a:hover { font-weight: 600; }",
    );

    assert_eq!(resolved.text.base().weight, 500);
    assert_eq!(
        resolved
            .text
            .resolve(brul_gui::WidgetState {
                hovered: true,
                ..brul_gui::WidgetState::default()
            })
            .weight,
        600
    );
}

#[test]
fn text_alignment_in_a_state_becomes_a_text_variant() {
    let resolved = resolved(".a { text-align: left; } .a:hover { text-align: right; }");

    assert_eq!(
        resolved
            .text
            .resolve(brul_gui::WidgetState {
                hovered: true,
                ..brul_gui::WidgetState::default()
            })
            .align,
        brul_gui::TextAlign::Right
    );
}

#[test]
fn vertical_alignment_reaches_the_base_and_the_state() {
    let resolved = resolved(
        ".a { width: 20px; height: 20px; vertical-align: bottom; } .a:hover { vertical-align: center; }",
    );

    assert_eq!(
        resolved.styles.base().vertical_align,
        brul_gui::VerticalAlign::Bottom
    );
    assert_eq!(
        resolved
            .styles
            .resolve(brul_gui::StyleState::Hovered.flags())
            .vertical_align,
        brul_gui::VerticalAlign::Center
    );
}

#[test]
fn states_layer_instead_of_replacing_each_other() {
    let resolved = resolved(
        ".a { background: #000000; border: 1px #111111; } .a:selected { border-color: #ff0000; } .a:hover { background: #00ff00; }",
    );

    let style = resolved.styles.resolve(brul_gui::WidgetState {
        hovered: true,
        selected: true,
        ..brul_gui::WidgetState::default()
    });

    assert_eq!(
        style.background,
        brul_gui::Paint::Color(brul_gui::Color::rgba(0, 255, 0, 255))
    );
    assert_eq!(style.border, brul_gui::Color::rgba(255, 0, 0, 255));
}

#[test]
fn animations_become_transitions_on_the_style_set() {
    let resolved =
        resolved(".a { background: #000000; animate background: 300ms; animate opacity: 50ms; }");
    let transitions = resolved.styles.transitions();

    assert_eq!(transitions.background, Some(Duration::from_millis(300)));
    assert_eq!(transitions.opacity, Some(Duration::from_millis(50)));
    assert_eq!(transitions.border, None);
    assert_eq!(transitions.shadow, None);
}

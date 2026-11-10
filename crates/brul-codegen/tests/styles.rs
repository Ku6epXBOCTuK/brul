use brul_codegen::styles::resolve;
use brul_gui::{Align, Dimension, Justify};

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
fn paint_covers_background_border_color_tint_and_the_gradient_refusal() {
    let resolved = resolved(
        ".a { background: #102030; border: 2px #405060; color: #708090; tint: #a0b0c0; radius: 6px; opacity: 0.5; }",
    );
    let style = resolved.styles.base();
    assert_eq!(style.background, brul_gui::Color::rgb(0x10, 0x20, 0x30));
    assert_eq!(style.border, brul_gui::Color::rgb(0x40, 0x50, 0x60));
    assert_eq!(style.text, brul_gui::Color::rgb(0x70, 0x80, 0x90));
    assert_eq!(style.tint, Some(brul_gui::Color::rgb(0xa0, 0xb0, 0xc0)));
    assert_eq!(style.radius, 6);
    assert_eq!(style.opacity, 0.5);
}

#[test]
fn a_gradient_background_is_refused() {
    let stylesheet =
        brul_syntax::style::parse_file(".a { background: linear-gradient(135deg, #fff, #000); }")
            .expect("stylesheet");
    let Err(error) = resolve(&stylesheet, &["a"], None) else {
        panic!("a gradient must not resolve");
    };
    assert!(error.contains("gradient"), "{error}");
}

#[test]
fn font_covers_size_family_weight_and_alignment() {
    let resolved = resolved(
        ".a { font-size: 20px; font-family: \"Inter\"; font-weight: 600; text-align: center; }",
    );
    assert_eq!(resolved.text.font_size, 20.0);
    assert_eq!(resolved.text.line_height, 24.0);
    assert_eq!(resolved.text.family.as_deref(), Some("Inter"));
    assert_eq!(resolved.text.weight, 600);
    assert_eq!(resolved.text.align, brul_gui::TextAlign::Center);
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

use std::rc::Rc;

use brul_gpu::{TEXTURE_INSTANCE_SIZE, texture_instance_bytes, texture_instances};
use brul_gui::{
    ClipRect, Color, Dimension, Dirty, DrawCommand, ExternalTexture, Image, LayoutStyle, Revision,
    Size, StyleSet, TextEngine, Texture, TextureFrame, Vector, Widget, WidgetKind, WidgetStyle,
    WidgetTree, draw_commands,
};

fn layout(widget: Widget) -> (TextEngine, brul_gui::WidgetSnapshot) {
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(widget);
    tree.layout(
        &mut engine,
        Size {
            width: 64,
            height: 64,
        },
        1.0,
    )
    .expect("layout");
    (engine, tree.snapshot().expect("snapshot").clone())
}

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn draw(snapshot: &brul_gui::WidgetSnapshot, dirty: &Dirty) -> Vec<DrawCommand> {
    let mut commands = Vec::new();
    draw_commands(snapshot, dirty, &mut commands);
    commands
}

#[test]
fn glyphs_become_mask_instances_tinted_with_the_text_color() {
    let text = Widget::text("A")
        .with_layout(fixed(32, 16))
        .with_styles(StyleSet::new(WidgetStyle {
            text: Color::rgb(255, 0, 0),
            ..WidgetStyle::default()
        }));
    let (engine, snapshot) = layout(text);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert!(!batches.glyphs.is_empty());
    assert!(batches.images.is_empty());
    for instance in &batches.glyphs {
        assert_eq!(instance.params[0], 0.0);
        assert_eq!(instance.tint, [1.0, 0.0, 0.0, 1.0]);
        assert!(instance.bounds[2] > 0.0 && instance.bounds[3] > 0.0);
        assert!(instance.uv[0] >= 0.0 && instance.uv[2] <= 1.0);
        assert!(instance.uv[1] >= 0.0 && instance.uv[3] <= 1.0);
    }
}

#[test]
fn a_clip_range_reaches_the_texture_instances() {
    let text = Widget::text("A").with_layout(fixed(32, 16));
    let (engine, mut snapshot) = layout(text);
    snapshot.nodes[0].clips = vec![ClipRect::new(1, 2, 30, 40, 5)];
    let clips = brul_gpu::clip_set(&snapshot, &Dirty::all());

    let batches = texture_instances(&draw(&snapshot, &Dirty::all()), engine.atlas(), &clips);

    assert!(!batches.glyphs.is_empty());
    for instance in &batches.glyphs {
        assert_eq!(instance.params[2], 0.0);
        assert_eq!(instance.params[3], 1.0);
    }
}

#[test]
fn a_tinted_image_is_one_color_instance_with_full_uv() {
    let texture = Texture::from_rgba(2, 2, &[255; 16]).expect("texture");
    let image = Widget::image(Rc::new(texture))
        .with_layout(fixed(16, 16))
        .with_styles(StyleSet::new(WidgetStyle {
            tint: Some(Color::rgb(255, 128, 0)),
            opacity: 0.5,
            ..WidgetStyle::default()
        }));
    let (engine, snapshot) = layout(image);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert!(batches.glyphs.is_empty());
    assert_eq!(batches.images.len(), 1);
    assert_eq!(batches.images[0].instances.len(), 1);
    let instance = &batches.images[0].instances[0];
    assert_eq!(instance.params, [1.0, 0.5, 0.0, 0.0]);
    assert_eq!(instance.tint, [1.0, 128.0 / 255.0, 0.0, 1.0]);
    assert_eq!(instance.uv, [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(instance.bounds[2..], [16.0, 16.0]);
}

#[test]
fn a_texture_tint_is_combined_with_the_style_tint() {
    let texture = Rc::new(
        Texture::from_rgba(1, 1, &[255; 4])
            .expect("texture")
            .with_tint(Color::rgb(255, 128, 0)),
    );
    let image = Widget::image(texture)
        .with_layout(fixed(8, 8))
        .with_styles(StyleSet::new(WidgetStyle {
            tint: Some(Color::rgb(128, 255, 255)),
            ..WidgetStyle::default()
        }));
    let (engine, snapshot) = layout(image);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert_eq!(
        batches.images[0].instances[0].tint,
        [128.0 / 255.0, 128.0 / 255.0, 0.0, 1.0]
    );
}

#[test]
fn an_svg_is_rasterized_on_cpu_and_kept_untinted() {
    const ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 6">
        <rect width="12" height="6" fill="currentColor"/>
    </svg>"#;
    let vector = Rc::new(Vector::from_svg(ICON).expect("vector"));
    let image = Widget::image(Image::Vector(vector))
        .with_layout(fixed(24, 12))
        .with_styles(StyleSet::new(WidgetStyle {
            text: Color::rgb(18, 52, 86),
            ..WidgetStyle::default()
        }));
    let (engine, snapshot) = layout(image);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert_eq!(batches.images.len(), 1);
    assert_eq!(batches.images[0].texture.width(), 24);
    assert_eq!(batches.images[0].texture.height(), 12);
    assert_eq!(
        batches.images[0].texture.color_at(0, 0),
        Some(Color::rgb(18, 52, 86))
    );
    assert_eq!(batches.images[0].instances[0].tint, [1.0; 4]);
}

#[test]
fn images_sharing_a_texture_share_one_batch() {
    let texture = Rc::new(Texture::from_rgba(1, 1, &[255, 0, 0, 255]).expect("texture"));
    let column = Widget::column(vec![
        Widget::image(Rc::clone(&texture)).with_layout(fixed(8, 8)),
        Widget::image(Rc::clone(&texture)).with_layout(fixed(8, 8)),
    ]);
    let (engine, snapshot) = layout(column);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert_eq!(batches.images.len(), 1);
    assert_eq!(batches.images[0].instances.len(), 2);
}

#[test]
fn zero_opacity_and_zero_sized_nodes_produce_no_instances() {
    let hidden = Widget::text("A")
        .with_layout(fixed(0, 0))
        .with_styles(StyleSet::new(WidgetStyle {
            opacity: 0.0,
            ..WidgetStyle::default()
        }));
    let (engine, snapshot) = layout(hidden);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert!(batches.glyphs.is_empty());
    assert!(batches.images.is_empty());
}

#[test]
fn a_dirty_filter_builds_only_changed_glyphs() {
    let text = Widget::text("A").with_layout(fixed(32, 16));
    let empty = Widget::column(Vec::new()).with_layout(fixed(8, 8));
    let (engine, snapshot) = layout(Widget::column(vec![empty, text]));
    let text_id = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Text)
        .expect("text")
        .id;
    let clips = brul_gpu::ClipSet::default();
    let none = Dirty {
        full: false,
        ids: Vec::new(),
    };
    let only_text = Dirty {
        full: false,
        ids: vec![text_id],
    };

    let none_batches = texture_instances(&draw(&snapshot, &none), engine.atlas(), &clips);
    let text_batches = texture_instances(&draw(&snapshot, &only_text), engine.atlas(), &clips);

    assert!(none_batches.glyphs.is_empty());
    assert!(!text_batches.glyphs.is_empty());
}

#[test]
fn instance_bytes_have_one_entry_per_instance() {
    let bytes = texture_instance_bytes(&[]);
    assert!(bytes.is_empty());

    let texture = Texture::from_rgba(1, 1, &[255; 4]).expect("texture");
    let image = Widget::image(texture).with_layout(fixed(8, 8));
    let (engine, snapshot) = layout(image);
    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    let bytes = texture_instance_bytes(&batches.images[0].instances);
    assert_eq!(bytes.len() as u64, TEXTURE_INSTANCE_SIZE);
}

struct ExternalRgba;

impl ExternalTexture for ExternalRgba {
    fn revision(&self) -> Revision {
        Revision::START
    }

    fn frame(&self) -> TextureFrame<'_> {
        TextureFrame::Rgba(&[255; 16])
    }
}

struct ExternalYuv;

impl ExternalTexture for ExternalYuv {
    fn revision(&self) -> Revision {
        Revision::START
    }

    fn frame(&self) -> TextureFrame<'_> {
        TextureFrame::Yuv420 {
            y: &[81; 4],
            u: &[128],
            v: &[128],
            strides: [2, 1, 1],
        }
    }
}

#[test]
fn an_external_rgba_frame_keeps_the_sample_mode() {
    let texture =
        Rc::new(Texture::external(2, 2, Rc::new(ExternalRgba)).expect("external texture"));
    let image = Widget::image(texture).with_layout(fixed(16, 16));
    let (engine, snapshot) = layout(image);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert_eq!(batches.images[0].instances[0].params[0], 1.0);
}

#[test]
fn an_external_yuv_frame_uses_the_conversion_mode() {
    let texture = Rc::new(Texture::external(2, 2, Rc::new(ExternalYuv)).expect("external texture"));
    let image = Widget::image(texture).with_layout(fixed(16, 16));
    let (engine, snapshot) = layout(image);

    let batches = texture_instances(
        &draw(&snapshot, &Dirty::all()),
        engine.atlas(),
        &brul_gpu::ClipSet::default(),
    );

    assert_eq!(batches.images[0].instances[0].params[0], 2.0);
}

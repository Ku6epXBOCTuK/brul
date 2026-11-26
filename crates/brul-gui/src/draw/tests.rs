use super::*;
use crate::{CaretPosition, SelectionRect, Size, TextEngine, TextStyle, Widget, WidgetTree};

fn input_snapshot() -> WidgetSnapshot {
    let root = Widget::column(vec![Widget::input("ab")]);
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 120,
            height: 80,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

fn input_node(snapshot: &mut WidgetSnapshot) -> &mut WidgetSnapshotNode {
    snapshot.nodes.last_mut().expect("input")
}

#[test]
fn vertical_offset_and_scroll_apply_to_glyphs_and_selection() {
    let mut snapshot = input_snapshot();
    let node = input_node(&mut snapshot);
    node.content_x = 5;
    node.content_y = 7;
    node.content_height = 40;
    node.scroll = 3;
    node.style.vertical_align = crate::VerticalAlign::Center;
    node.selection = vec![SelectionRect {
        x: 2,
        y: 4,
        width: 6,
        height: 8,
    }];
    let text = node.text.as_ref().expect("text");
    let first = text.glyphs[0];
    let height = text.size.height;

    let mut commands = Vec::new();
    draw_commands(&snapshot, &Dirty::all(), &mut commands);

    let offset = crate::VerticalAlign::Center.offset(40, height) as i32 - 3;
    let glyph = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Glyph(glyph) => Some(*glyph),
            _ => None,
        })
        .expect("glyph");
    assert_eq!((glyph.x, glyph.y), (5 + first.x, 7 + first.y + offset));
    let selection = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Rect(rect) => Some(*rect),
            _ => None,
        })
        .expect("selection");
    assert_eq!(selection.rect[1], 7 + 4 + offset);
}

#[test]
fn preedit_shift_uses_the_caret_x() {
    let mut snapshot = input_snapshot();
    let mut engine = TextEngine::new();
    let preedit = Rc::new(
        engine
            .layout("abc", &TextStyle::new(10.0), None, 1.0)
            .expect("preedit"),
    );
    let node = input_node(&mut snapshot);
    node.content_x = 10;
    node.caret = Some(CaretPosition {
        x: 30,
        y: 2,
        height: 7,
    });
    node.preedit = Some(Rc::clone(&preedit));
    let last = preedit.glyphs.last().expect("preedit glyph");
    let shift = 30 - preedit.size.width as i32;

    let mut commands = Vec::new();
    draw_commands(&snapshot, &Dirty::all(), &mut commands);

    let glyph = commands
        .iter()
        .rev()
        .find_map(|command| match command {
            DrawCommand::Glyph(glyph) => Some(*glyph),
            _ => None,
        })
        .expect("preedit glyph");
    assert_eq!(glyph.x, 10 + shift + last.x);
    let underline = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Rect(rect) if rect.rect[2] == preedit.size.width as i32 => Some(*rect),
            _ => None,
        })
        .expect("underline");
    assert_eq!(underline.rect[0], 10 + shift);
}

#[test]
fn caret_without_text_uses_a_clamped_height() {
    let mut snapshot = input_snapshot();
    let node = input_node(&mut snapshot);
    node.text = None;
    node.content_y = 5;
    node.content_height = 30;
    node.scroll = 2;
    node.style.vertical_align = crate::VerticalAlign::Bottom;
    node.caret = Some(CaretPosition {
        x: 1,
        y: -4,
        height: 6,
    });

    let mut commands = Vec::new();
    draw_commands(&snapshot, &Dirty::all(), &mut commands);

    let caret = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Rect(rect) => Some(*rect),
            _ => None,
        })
        .expect("caret");
    let offset = crate::VerticalAlign::Bottom.offset(30, 0) as i32 - 2;
    assert_eq!(caret.rect[1], 5 - 4 + offset);
}

#[test]
fn preedit_without_caret_emits_no_composition_commands() {
    let mut snapshot = input_snapshot();
    let node = input_node(&mut snapshot);
    let mut engine = TextEngine::new();
    node.preedit = Some(Rc::new(
        engine
            .layout("abc", &TextStyle::new(10.0), None, 1.0)
            .expect("preedit"),
    ));
    node.caret = None;
    let glyph_count = node.text.as_ref().expect("text").glyphs.len();

    let mut commands = Vec::new();
    draw_commands(&snapshot, &Dirty::all(), &mut commands);

    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Glyph(_)))
            .count(),
        glyph_count
    );
    assert!(
        commands
            .iter()
            .all(|command| !matches!(command, DrawCommand::Rect(_)))
    );
}

#[test]
fn dirty_limits_commands_to_the_listed_node() {
    let snapshot = input_snapshot();
    let first = snapshot.nodes[0].id;
    let dirty = Dirty {
        full: false,
        ids: vec![first],
    };

    let mut commands = Vec::new();
    draw_commands(&snapshot, &dirty, &mut commands);

    assert!(commands.iter().all(|command| match command {
        DrawCommand::Shape(shape) => shape.node == 0,
        DrawCommand::Glyph(glyph) => glyph.node == 0,
        DrawCommand::Rect(rect) => rect.node == 0,
        DrawCommand::Image(image) => image.node == 0,
    }));
}

#[test]
fn image_tint_is_resolved_once() {
    let texture = Texture::from_rgba(1, 1, &[255, 255, 255, 255])
        .expect("texture")
        .with_tint(Color::rgb(10, 20, 30));
    let mut snapshot = input_snapshot();
    let node = input_node(&mut snapshot);
    node.kind = WidgetKind::Image;
    node.content_width = 4;
    node.content_height = 5;
    node.style.tint = Some(Color::rgb(40, 50, 60));
    node.image = Some(Image::from(texture));

    let mut commands = Vec::new();
    draw_commands(&snapshot, &Dirty::all(), &mut commands);

    let image = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Image(image) => Some(image),
            _ => None,
        })
        .expect("image");
    assert_eq!(
        image.tint,
        Some(Color::rgb(10, 20, 30).multiply(Color::rgb(40, 50, 60)))
    );
}

use std::rc::Rc;
use std::time::Duration;

use brul_gui::{
    CanvasPainter, CanvasSpec, Color, Dimension, Dirty, DrawCommand, LayoutStyle, PaintCommand,
    PaintCtx, PaintRect, Size, StyleSet, TextEngine, Widget, WidgetStyle, WidgetTree,
    draw_commands,
};

struct Probe;

impl CanvasPainter for Probe {
    fn paint(&self, ctx: &PaintCtx<'_>) -> Vec<PaintCommand> {
        vec![PaintCommand::Rect(PaintRect {
            rect: [
                ctx.rect.x,
                ctx.rect.y,
                ctx.rect.width as i32,
                ctx.rect.height as i32,
            ],
            color: Color::rgb(255, 0, 0),
        })]
    }
}

fn canvas(spec: CanvasSpec, opacity: f32) -> Widget {
    Widget::canvas(spec)
        .with_layout(LayoutStyle {
            width: Dimension::Px(32),
            height: Dimension::Px(24),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            opacity,
            ..WidgetStyle::default()
        }))
}

fn tree(root: Widget) -> WidgetTree {
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 64,
            height: 64,
        },
        1.0,
    )
    .expect("layout");
    tree.take_dirty();
    tree
}

#[test]
fn painter_commands_get_node_index_and_opacity() {
    let canvas = canvas(CanvasSpec::new(), 0.5);
    let id = canvas.id();
    let mut tree = tree(Widget::column(vec![canvas]));
    tree.set_painter(id, Rc::new(Probe));

    let snapshot = tree.snapshot().expect("snapshot");
    let mut commands = Vec::new();
    draw_commands(snapshot, &Dirty::all(), &mut commands);

    match commands.as_slice() {
        [DrawCommand::Rect(command)] => {
            assert_eq!(command.node, 1);
            assert_eq!(command.rect, [0, 0, 32, 24]);
            assert_eq!(command.color, Color::rgb(255, 0, 0));
            assert_eq!(command.opacity, 0.5);
        }
        other => panic!("expected one canvas rect, got {} commands", other.len()),
    }
}

#[test]
fn canvas_clips_painter_output_to_content() {
    let canvas = canvas(CanvasSpec::new(), 1.0);
    let id = canvas.id();
    let mut tree = tree(Widget::column(vec![canvas]));
    tree.set_painter(id, Rc::new(Probe));

    let snapshot = tree.snapshot().expect("snapshot");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("canvas");
    assert_eq!(node.clips.len(), 1);
    assert!(node.canvas.is_some());
}

#[test]
fn painted_canvas_is_dirty_only_when_marked() {
    let canvas = canvas(CanvasSpec::new(), 1.0);
    let id = canvas.id();
    let mut tree = tree(Widget::column(vec![canvas]));
    tree.set_painter(id, Rc::new(Probe));
    let dirty = tree.take_dirty();
    assert!(!dirty.full);
    assert!(dirty.ids.contains(&id));

    let snapshot = tree.snapshot().expect("snapshot");
    let mut commands = Vec::new();
    draw_commands(
        snapshot,
        &Dirty {
            full: false,
            ids: Vec::new(),
        },
        &mut commands,
    );
    assert!(commands.is_empty());

    draw_commands(
        snapshot,
        &Dirty {
            full: false,
            ids: vec![id],
        },
        &mut commands,
    );
    assert!(matches!(commands.as_slice(), [DrawCommand::Rect(_)]));
}

#[test]
fn removing_painter_drops_canvas_commands() {
    let canvas = canvas(CanvasSpec::new(), 1.0);
    let id = canvas.id();
    let mut tree = tree(Widget::column(vec![canvas]));
    tree.set_painter(id, Rc::new(Probe));
    let snapshot = tree.snapshot().expect("snapshot");
    let mut commands = Vec::new();
    draw_commands(snapshot, &Dirty::all(), &mut commands);
    assert!(!commands.is_empty());

    tree.remove_painter(id);
    let snapshot = tree.snapshot().expect("snapshot");
    let mut commands = Vec::new();
    draw_commands(snapshot, &Dirty::all(), &mut commands);
    assert!(commands.is_empty());
}

#[test]
fn animated_canvas_keeps_frames() {
    let canvas_widget = canvas(CanvasSpec::new().animated(true), 1.0);
    let id = canvas_widget.id();
    let mut canvas_tree = tree(Widget::column(vec![canvas_widget]));
    assert!(canvas_tree.animating());
    assert!(canvas_tree.advance(Duration::from_millis(16)));
    assert!(canvas_tree.take_dirty().ids.contains(&id));

    let plain = canvas(CanvasSpec::new(), 1.0);
    let plain_tree = tree(Widget::column(vec![plain]));
    assert!(!plain_tree.animating());
}

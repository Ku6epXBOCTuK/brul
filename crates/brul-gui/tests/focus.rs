use brul_gui::{
    Color, Dimension, Direction, LayoutStyle, Paint, Size, StyleSet, StyleState, TextEngine,
    Widget, WidgetStyle, WidgetStylePatch, WidgetTree,
};

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn laid_out(root: Widget) -> WidgetTree {
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    tree
}

fn field() -> Widget {
    Widget::view(Direction::Column, vec![])
        .with_layout(fixed(40, 20))
        .focusable(true)
}

#[test]
fn focus_applies_the_focused_style_and_blur_removes_it() {
    let base = Paint::Color(Color::rgb(1, 2, 3));
    let focused = Paint::Color(Color::rgb(4, 5, 6));
    let field = field().with_styles(
        StyleSet::new(WidgetStyle {
            background: base,
            ..WidgetStyle::default()
        })
        .with(
            StyleState::Focused,
            WidgetStylePatch {
                background: Some(focused),
                ..WidgetStylePatch::default()
            },
        ),
    );
    let id = field.id();
    let mut tree = laid_out(Widget::column(vec![field]).with_layout(fixed(100, 100)));

    tree.set_focused(Some(id));
    {
        let snapshot = tree.snapshot().expect("snapshot");
        let node = snapshot.node(id).expect("field");
        assert!(node.state.focused);
        assert_eq!(node.style.background, focused);
    }

    tree.set_focused(None);
    let snapshot = tree.snapshot().expect("snapshot after blur");
    let node = snapshot.node(id).expect("field after blur");
    assert!(!node.state.focused);
    assert_eq!(node.style.background, base);
}

#[test]
fn replacing_the_same_children_keeps_the_focus_on_its_widget() {
    let (first, second) = (field(), field());
    let (first_id, second_id) = (first.id(), second.id());
    let root = Widget::column(vec![first, second]).with_layout(fixed(100, 100));
    let root_id = root.id();
    let mut tree = laid_out(root);

    tree.set_focused(Some(first_id));
    let children = tree.take_children(root_id).expect("children");
    assert!(tree.replace_all_children(root_id, children));

    assert_eq!(tree.focused(), Some(first_id));
    {
        let snapshot = tree.snapshot().expect("snapshot");
        assert!(snapshot.node(first_id).expect("first").state.focused);
        assert!(!snapshot.node(second_id).expect("second").state.focused);
    }
}

use std::time::Duration;

use brul_gui::{
    Color, Dimension, LayoutStyle, Paint, Size, StyleSet, StyleState, TextEngine, Transitions,
    Widget, WidgetStyle, WidgetTree,
};

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn style(background: Color) -> WidgetStyle {
    WidgetStyle {
        background: Paint::Color(background),
        ..WidgetStyle::default()
    }
}

fn hover_tree() -> (WidgetTree, brul_gui::WidgetId) {
    let transitions = Transitions {
        background: Some(Duration::from_millis(300)),
        ..Transitions::default()
    };
    let base = style(Color::BLACK);
    let hovered = style(Color::WHITE);
    let view = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(
            StyleSet::new(base)
                .with(
                    StyleState::Hovered,
                    brul_gui::WidgetStylePatch {
                        background: Some(hovered.background),
                        ..brul_gui::WidgetStylePatch::default()
                    },
                )
                .with_transitions(transitions),
        );
    let id = view.id();
    let mut tree = WidgetTree::new(view);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 50,
            height: 50,
        },
        1.0,
    )
    .expect("layout");
    (tree, id)
}

fn background(tree: &mut WidgetTree, id: brul_gui::WidgetId) -> Paint {
    tree.snapshot()
        .expect("snapshot")
        .node(id)
        .expect("view")
        .style
        .background
}

#[test]
fn a_transition_slows_the_hover_change() {
    let (mut tree, id) = hover_tree();
    tree.set_hovered(id, true);
    assert!(tree.animating());
    assert_eq!(background(&mut tree, id), Paint::Color(Color::BLACK));

    assert!(tree.advance(Duration::from_millis(150)));
    let middle = background(&mut tree, id);
    assert_ne!(middle, Paint::Color(Color::BLACK));
    assert_ne!(middle, Paint::Color(Color::WHITE));

    assert!(!tree.advance(Duration::from_millis(150)));
    assert_eq!(background(&mut tree, id), Paint::Color(Color::WHITE));
    assert!(!tree.animating());
}

#[test]
fn an_interrupted_transition_continues_from_where_it_is() {
    let (mut tree, id) = hover_tree();
    tree.set_hovered(id, true);
    tree.advance(Duration::from_millis(150));
    let middle = background(&mut tree, id);

    tree.set_hovered(id, false);
    assert_eq!(background(&mut tree, id), middle);

    tree.advance(Duration::from_millis(150));
    let back = background(&mut tree, id);
    assert_ne!(back, middle);
    assert_ne!(back, Paint::Color(Color::BLACK));
    assert_ne!(back, Paint::Color(Color::WHITE));

    tree.advance(Duration::from_millis(150));
    assert_eq!(background(&mut tree, id), Paint::Color(Color::BLACK));
}

#[test]
fn removing_an_animating_widget_stops_animation() {
    let transitions = Transitions {
        background: Some(Duration::from_millis(300)),
        ..Transitions::default()
    };
    let child = Widget::column(vec![])
        .with_layout(fixed(50, 50))
        .with_styles(
            StyleSet::new(style(Color::BLACK))
                .with(
                    StyleState::Hovered,
                    brul_gui::WidgetStylePatch {
                        background: Some(Paint::Color(Color::WHITE)),
                        ..brul_gui::WidgetStylePatch::default()
                    },
                )
                .with_transitions(transitions),
        );
    let child_id = child.id();
    let root = Widget::column(vec![child]);
    let root_id = root.id();
    let mut tree = WidgetTree::new(root);
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 50,
            height: 50,
        },
        1.0,
    )
    .expect("layout");
    tree.set_hovered(child_id, true);
    assert!(tree.animating());

    assert!(tree.replace_children(root_id, 0, 1, vec![]));
    assert!(!tree.animating());
}

#[test]
fn replacing_the_root_stops_animation() {
    let (mut tree, id) = hover_tree();
    tree.set_hovered(id, true);
    assert!(tree.animating());

    tree.replace_root(Widget::column(vec![]));
    assert!(!tree.animating());
}

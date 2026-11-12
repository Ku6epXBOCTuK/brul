use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::{
    Color, Dimension, LayoutStyle, Paint, Size, StyleSet, StyleState, TextEngine, Widget,
    WidgetStyle, WidgetStylePatch, WidgetTree,
};
use brul_reactive::{Effect, Signal, batch};

#[test]
fn signal_change_reaches_widget_state_snapshot_and_dirty_set() {
    let base = Color::rgb(10, 10, 10);
    let checked = Color::rgb(0, 120, 255);
    let control = Widget::column(Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(40),
            height: Dimension::Px(40),
            ..LayoutStyle::default()
        })
        .with_styles(
            StyleSet::new(WidgetStyle {
                background: Paint::Color(base),
                ..WidgetStyle::default()
            })
            .with(
                StyleState::Checked,
                WidgetStylePatch {
                    background: Some(Paint::Color(checked)),
                    ..WidgetStylePatch::default()
                },
            ),
        );
    let id = control.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(control)));
    let signal = Signal::new(false);
    let _effect = Effect::new({
        let tree = Rc::clone(&tree);
        move || {
            tree.borrow_mut().set_checked(id, signal.get());
        }
    });

    let mut engine = TextEngine::new();
    tree.borrow_mut()
        .layout(
            &mut engine,
            Size {
                width: 40,
                height: 40,
            },
            1.0,
        )
        .expect("layout");
    let _ = tree.borrow_mut().take_dirty();
    {
        let mut tree = tree.borrow_mut();
        let snapshot = tree.snapshot().expect("snapshot");
        assert_eq!(
            snapshot.node(id).expect("control").style.background,
            Paint::Color(base)
        );
    }

    batch(|| signal.set(true));

    let dirty = tree.borrow_mut().take_dirty();
    assert!(!dirty.full);
    assert!(dirty.ids.contains(&id));
    let mut tree = tree.borrow_mut();
    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(
        snapshot.node(id).expect("control").style.background,
        Paint::Color(checked)
    );
}

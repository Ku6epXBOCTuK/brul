use std::time::Duration;

use brul_gui::{EffectSpec, Size, TextEngine, Widget, WidgetTree};

fn effect(animated: bool) -> EffectSpec {
    EffectSpec {
        name: "cover",
        params: [[0.0; 4]; 16],
        animated,
    }
}

#[test]
fn effect_reaches_the_snapshot() {
    let child = Widget::column(Vec::new()).effect(effect(false));
    let mut tree = WidgetTree::new(Widget::column(vec![child]));
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 16,
            height: 16,
        },
        1.0,
    )
    .expect("layout");

    let snapshot = tree.snapshot().expect("snapshot");
    assert_eq!(snapshot.nodes.len(), 2);
    assert_eq!(
        snapshot.nodes[1].effect.as_ref().map(|e| e.name),
        Some("cover")
    );
    assert_eq!(snapshot.nodes[0].effect, None);
}

#[test]
fn animated_effects_stay_dirty() {
    let child = Widget::column(Vec::new()).effect(effect(true));
    let id = child.id();
    let mut tree = WidgetTree::new(Widget::column(vec![child]));

    assert!(tree.animating());
    assert!(tree.take_dirty().full);
    assert!(tree.advance(Duration::from_millis(16)));
    assert!(tree.take_dirty().ids.contains(&id));
}

use brul_reactive::{Effect, EffectScope, live_nodes};

fn add_effect(scope: &EffectScope) {
    scope.run(|| {
        Effect::new(|| {});
    });
}

#[test]
fn window_removal_releases_nested_component_effects() {
    let baseline = live_nodes();
    let window = EffectScope::new();
    let component = window.child();
    add_effect(&component);
    assert_eq!(live_nodes(), baseline + 1);

    window.dispose();
    assert_eq!(live_nodes(), baseline);
}

#[test]
fn if_branch_removal_releases_its_effects() {
    let baseline = live_nodes();
    let component = EffectScope::new();
    add_effect(&component);
    let branch = component.child();
    add_effect(&branch);
    assert_eq!(live_nodes(), baseline + 2);

    branch.dispose();
    assert_eq!(live_nodes(), baseline + 1);
}

#[test]
fn each_row_removal_releases_its_effect() {
    let baseline = live_nodes();
    let component = EffectScope::new();
    add_effect(&component);
    let row = component.child();
    add_effect(&row);
    assert_eq!(live_nodes(), baseline + 2);

    drop(row);
    assert_eq!(live_nodes(), baseline + 1);
}

#[test]
fn component_removal_releases_own_and_descendant_effects() {
    let baseline = live_nodes();
    let window = EffectScope::new();
    add_effect(&window);
    let component = window.child();
    add_effect(&component);
    let branch = component.child();
    add_effect(&branch);
    let row = component.child();
    add_effect(&row);
    assert_eq!(live_nodes(), baseline + 4);

    component.dispose();
    assert_eq!(live_nodes(), baseline + 1);
    window.dispose();
    assert_eq!(live_nodes(), baseline);
}

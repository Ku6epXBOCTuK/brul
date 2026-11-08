use bropicker::Theme;
use bropicker::ui::{PickerIds, PickerView, apply_events, build_view, demo_browsers, palette};
use brul_gui::{Size, TextEngine, WidgetId, WidgetTree};

fn build(
    theme: Theme,
    browsers: &[bropicker::BrowserEntry],
    remember_choice: bool,
    always_ask: bool,
) -> (WidgetTree, PickerIds) {
    let selected = browsers.iter().position(|b| b.is_default).unwrap_or(0);
    let PickerView { root, ids } = build_view(
        browsers,
        "https://example.com/page",
        theme,
        selected,
        remember_choice,
        always_ask,
    );
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 488,
            height: 800,
        },
        1.0,
    )
    .expect("layout");

    (tree, ids)
}

fn center(tree: &mut WidgetTree, id: WidgetId) -> (i32, i32) {
    let node = tree.snapshot().expect("snapshot").node(id).expect("node");
    (
        node.x + node.width as i32 / 2,
        node.y + node.height as i32 / 2,
    )
}

fn click(tree: &mut WidgetTree, x: i32, y: i32) -> Vec<brul_gui::WidgetEvent> {
    let mut events = tree.pointer_down(x, y).expect("pointer down");
    events.extend(tree.pointer_up(x, y).expect("pointer up"));
    events
}

#[test]
fn dark_picker_fills_the_window() {
    let browsers = demo_browsers();
    let (mut tree, _) = build(Theme::Dark, &browsers, true, true);
    let snapshot = tree.snapshot().expect("snapshot").clone();

    assert_eq!(
        snapshot.size,
        Size {
            width: 488,
            height: 800
        }
    );
    assert_eq!(
        snapshot.nodes[0].style.background,
        palette(Theme::Dark).background
    );
    assert!(
        snapshot
            .nodes
            .iter()
            .filter(|node| node.text.is_some())
            .count()
            >= 10
    );
}

#[test]
fn light_theme_uses_its_own_palette() {
    let browsers = demo_browsers();
    let (mut dark, _) = build(Theme::Dark, &browsers, true, true);
    let (mut light, _) = build(Theme::Light, &browsers, true, true);

    let dark_background = dark.snapshot().expect("dark").nodes[0].style.background;
    let light_background = light.snapshot().expect("light").nodes[0].style.background;

    assert_eq!(light_background, palette(Theme::Light).background);
    assert_ne!(dark_background, light_background);
}

#[test]
fn default_badge_is_only_on_the_default_browser() {
    let mut without_default = demo_browsers();
    for browser in &mut without_default {
        browser.is_default = false;
    }

    let (mut plain, _) = build(Theme::Dark, &without_default, true, true);
    let (mut with_default, _) = build(Theme::Dark, &demo_browsers(), true, true);
    let plain_texts = plain
        .snapshot()
        .expect("plain")
        .nodes
        .iter()
        .filter(|node| node.text.is_some())
        .count();
    let default_texts = with_default
        .snapshot()
        .expect("default")
        .nodes
        .iter()
        .filter(|node| node.text.is_some())
        .count();

    assert_eq!(default_texts, plain_texts + 1);
}

#[test]
fn explicit_selection_is_kept() {
    let browsers = demo_browsers();
    let selected = 2;
    let PickerView { root, ids } = build_view(
        &browsers,
        "https://example.com/page",
        Theme::Dark,
        selected,
        true,
        true,
    );
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(root);
    tree.layout(
        &mut engine,
        Size {
            width: 488,
            height: 800,
        },
        1.0,
    )
    .expect("layout");

    assert!(tree.state(ids.rows[selected]).expect("row").selected);
    assert!(!tree.state(ids.rows[0]).expect("row").selected);
}

#[test]
fn default_browser_starts_selected() {
    let browsers = demo_browsers();
    let (tree, ids) = build(Theme::Dark, &browsers, true, true);

    assert!(tree.state(ids.rows[0]).expect("first row").selected);
    assert!(!tree.state(ids.rows[1]).expect("second row").selected);
}

#[test]
fn click_selects_row_and_updates_style() {
    let browsers = demo_browsers();
    let (mut tree, ids) = build(Theme::Dark, &browsers, true, true);
    let (x, y) = center(&mut tree, ids.rows[2]);

    let events = click(&mut tree, x, y);
    let selected = apply_events(&mut tree, &ids, &events);

    assert_eq!(selected, Some(2));
    assert!(tree.state(ids.rows[2]).expect("third row").selected);
    assert!(!tree.state(ids.rows[0]).expect("first row").selected);

    let background = tree
        .snapshot()
        .expect("snapshot")
        .node(ids.rows[2])
        .expect("row node")
        .style
        .background;
    assert_eq!(background, palette(Theme::Dark).selected);
}

#[test]
fn hover_changes_row_style() {
    let browsers = demo_browsers();
    let (mut tree, ids) = build(Theme::Dark, &browsers, true, true);
    let (x, y) = center(&mut tree, ids.rows[2]);

    tree.pointer_moved(x, y).expect("pointer moved");

    assert!(tree.state(ids.rows[2]).expect("third row").hovered);
    let background = tree
        .snapshot()
        .expect("snapshot")
        .node(ids.rows[2])
        .expect("row node")
        .style
        .background;
    assert_eq!(background, palette(Theme::Dark).hover);
}

#[test]
fn checkbox_click_toggles_mark_and_row() {
    let browsers = demo_browsers();
    let (mut tree, ids) = build(Theme::Dark, &browsers, false, true);
    let (x, y) = center(&mut tree, ids.remember.mark);

    let events = click(&mut tree, x, y);
    apply_events(&mut tree, &ids, &events);

    assert!(tree.state(ids.remember.mark).expect("mark").checked);
    assert!(tree.state(ids.remember.row).expect("row").checked);

    let background = tree
        .snapshot()
        .expect("snapshot")
        .node(ids.remember.mark)
        .expect("mark node")
        .style
        .background;
    assert_eq!(background, palette(Theme::Dark).accent);
}

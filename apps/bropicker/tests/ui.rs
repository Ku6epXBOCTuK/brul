use bropicker::Theme;
use bropicker::ui::{build_tree, demo_browsers, palette};
use brul_gui::{Size, TextEngine, WidgetSnapshot, WidgetTree};

fn snapshot(theme: Theme, browsers: &[bropicker::BrowserEntry]) -> WidgetSnapshot {
    let mut engine = TextEngine::new();
    let mut tree = WidgetTree::new(build_tree(browsers, "https://github.com/", theme));
    tree.layout(
        &mut engine,
        Size {
            width: 488,
            height: 800,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

#[test]
fn dark_picker_fills_the_window() {
    let snapshot = snapshot(Theme::Dark, &demo_browsers());

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
    let snapshot = snapshot(Theme::Light, &demo_browsers());

    assert_eq!(
        snapshot.nodes[0].style.background,
        palette(Theme::Light).background
    );
    assert_ne!(
        palette(Theme::Light).background,
        palette(Theme::Dark).background
    );
}

#[test]
fn default_badge_is_only_on_the_default_browser() {
    let mut browsers = demo_browsers();
    for browser in &mut browsers {
        browser.is_default = false;
    }
    let without = snapshot(Theme::Dark, &browsers);

    browsers[0].is_default = true;
    let with = snapshot(Theme::Dark, &browsers);

    let without_text = without
        .nodes
        .iter()
        .filter(|node| node.text.is_some())
        .count();
    let with_text = with.nodes.iter().filter(|node| node.text.is_some()).count();
    assert_eq!(with_text, without_text + 1);
}

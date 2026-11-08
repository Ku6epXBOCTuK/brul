use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use bropicker::Config;
use bropicker::ui::{PickerView, apply_events, build_view, demo_browsers};
use brul_gui::WidgetTree;
use brul_gui::window::{WindowConfig, run_tree};

fn main() {
    if let Err(error) = run() {
        eprintln!("bropicker: {error}");
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://github.com/".to_string());
    let config = Config::load_default()?;
    let theme = config.theme;
    let remember_choice = config.settings.remember_choice;
    let always_ask = config.settings.always_ask;
    let browsers = if config.browsers.is_empty() {
        demo_browsers()
    } else {
        config.browsers
    };

    let PickerView { root, ids } = build_view(&browsers, &url, theme, remember_choice, always_ask);
    let names: Vec<String> = browsers
        .iter()
        .map(|browser| browser.name.clone())
        .collect();
    let tree = Rc::new(RefCell::new(WidgetTree::new(root)));
    let handler_tree = Rc::clone(&tree);

    run_tree(
        WindowConfig {
            title: "bropicker".to_string(),
            width: 488,
            height: 800,
            header_height: 72,
        },
        tree,
        move |events| {
            let mut tree = handler_tree.borrow_mut();
            if let Some(index) = apply_events(&mut tree, &ids, events)
                && let Some(name) = names.get(index)
            {
                println!("selected: {name}");
            }
        },
    )?;

    Ok(())
}

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use bropicker::Config;
use bropicker::ui::{build_tree, demo_browsers};
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
        .unwrap_or_else(|| String::from("https://github.com/"));
    let config = Config::load_default()?;
    let theme = config.theme;
    let browsers = if config.browsers.is_empty() {
        demo_browsers()
    } else {
        config.browsers
    };

    let tree = Rc::new(RefCell::new(WidgetTree::new(build_tree(
        &browsers, &url, theme,
    ))));
    let window = WindowConfig {
        title: String::from("bropicker"),
        width: 488,
        height: 800,
        header_height: 72,
    };

    run_tree(window, tree, |_| {})?;
    Ok(())
}

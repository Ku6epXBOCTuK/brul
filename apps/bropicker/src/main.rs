use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use bropicker::launcher::Launcher;
use bropicker::ui::{PickerIds, apply_events, build_view, demo_browsers};
use bropicker::{Config, Theme};
use brul_gui::window::{ColorScheme, WindowConfig, run_tree_themed};
use brul_gui::{WidgetEvent, WidgetTree};

struct AppState {
    tree: Rc<RefCell<WidgetTree>>,
    ids: PickerIds,
    selected: usize,
    remember: bool,
    always_ask: bool,
}

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
    let configured_theme = config.theme;
    let browsers = if config.browsers.is_empty() {
        demo_browsers()
    } else {
        config.browsers.clone()
    };
    let selected = browsers
        .iter()
        .position(|browser| browser.is_default)
        .unwrap_or(0);
    let remember = config.settings.remember_choice;
    let always_ask = config.settings.always_ask;

    let initial = build_view(
        &browsers,
        &url,
        effective_theme(configured_theme, ColorScheme::Dark),
        selected,
        remember,
        always_ask,
    );
    let state = Rc::new(RefCell::new(AppState {
        tree: Rc::new(RefCell::new(WidgetTree::new(initial.root))),
        ids: initial.ids,
        selected,
        remember,
        always_ask,
    }));

    let build_state = Rc::clone(&state);
    let build_browsers = browsers.clone();
    let build_url = url.clone();
    let build = move |scheme: ColorScheme| {
        let mut state = build_state.borrow_mut();
        let theme = effective_theme(configured_theme, scheme);
        let view = build_view(
            &build_browsers,
            &build_url,
            theme,
            state.selected,
            state.remember,
            state.always_ask,
        );
        state.ids = view.ids;
        let tree = Rc::new(RefCell::new(WidgetTree::new(view.root)));
        state.tree = Rc::clone(&tree);
        tree
    };

    let handler_state = Rc::clone(&state);
    let handler_browsers = browsers.clone();
    let handler_url = url.clone();
    let handler = move |events: &[WidgetEvent]| {
        let (tree, ids) = {
            let state = handler_state.borrow();
            (Rc::clone(&state.tree), state.ids.clone())
        };
        let mut tree = tree.borrow_mut();
        let selected = apply_events(&mut tree, &ids, events);
        let remember = tree
            .state(ids.remember.row)
            .is_some_and(|state| state.checked);
        let always_ask = tree
            .state(ids.always_ask.row)
            .is_some_and(|state| state.checked);
        drop(tree);

        {
            let mut state = handler_state.borrow_mut();
            state.remember = remember;
            state.always_ask = always_ask;
            if let Some(index) = selected {
                state.selected = index;
            }
        }

        if let Some(index) = selected
            && let Some(browser) = handler_browsers.get(index)
            && let Err(error) = Launcher::new().launch(browser, &handler_url)
        {
            eprintln!("launch: {error}");
        }
    };

    run_tree_themed(
        WindowConfig {
            title: "bropicker".to_string(),
            width: 488,
            height: 800,
            header_height: 72,
        },
        build,
        handler,
    )?;

    Ok(())
}

fn effective_theme(configured: Theme, scheme: ColorScheme) -> Theme {
    match configured {
        Theme::System => match scheme {
            ColorScheme::Light => Theme::Light,
            ColorScheme::Dark => Theme::Dark,
        },
        theme => theme,
    }
}

#[cfg(test)]
mod tests {
    use super::effective_theme;
    use bropicker::Theme;
    use brul_gui::window::ColorScheme;

    #[test]
    fn system_theme_follows_scheme() {
        assert_eq!(
            effective_theme(Theme::System, ColorScheme::Light),
            Theme::Light
        );
        assert_eq!(
            effective_theme(Theme::System, ColorScheme::Dark),
            Theme::Dark
        );
    }

    #[test]
    fn manual_theme_ignores_scheme() {
        assert_eq!(
            effective_theme(Theme::Light, ColorScheme::Dark),
            Theme::Light
        );
        assert_eq!(
            effective_theme(Theme::Dark, ColorScheme::Light),
            Theme::Dark
        );
    }
}

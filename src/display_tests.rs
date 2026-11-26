use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;

use brul_display_test::{Action, Display, Meta};
use brul_gui::{KeyModifiers, KeyPress, RawEvent, RawUiEvent};
use brul_window::WindowCommand;

use crate::ui::AssemblyParts;
use crate::{Build, Script, Theme, Ui, WindowDef};

trait Generated {
    type Theme: Theme + FromStr;

    fn parse_theme(name: &str) -> Result<Self::Theme, <Self::Theme as FromStr>::Err>;
    fn windows() -> Vec<WindowDef<Self::Theme>>;
    fn names() -> &'static [&'static str];
    fn script() -> Option<Script>;
}

fn run_case<G: Generated>(name: &str, source: &str, expected: &[u8], expected_path: &str) {
    let meta = Meta::parse(source).unwrap_or_else(|error| panic!("{name}: {error}"));
    let theme = meta.theme.as_deref().map(|value| {
        G::parse_theme(value).unwrap_or_else(|_| panic!("{name}: unknown theme {value}"))
    });
    let component =
        crate::run::generated_component(G::names(), G::script(), &[]).expect("generated component");
    let mut build = Build::new();
    let view = G::windows()[0].view;
    let built = view(theme, &mut build, &component).expect("generated view");
    let commands = Rc::new(RefCell::new(Vec::<WindowCommand>::new()));
    let mut ui = Ui::assemble_with(
        Rc::clone(&component),
        built,
        view,
        theme,
        AssemblyParts {
            structures: build.take_structures(),
            capabilities: build.take_capabilities(),
            nested: build.take_components(),
            states: build.take_states(),
            event_sink: None,
        },
        Rc::clone(&commands),
        true,
    );
    ui.dispatch(&[]);
    ui.settle();

    let mut display = Display::new(meta.size);
    ui.tree
        .borrow_mut()
        .layout(display.engine_mut(), meta.size, 1.0)
        .expect("layout");

    for action in &meta.actions {
        match action {
            Action::Shot(label) => {
                let mut tree = ui.tree.borrow_mut();
                display.paint_all(&mut tree);
                display.shot(label);
            }
            Action::Hover(x, y) => {
                let events = ui
                    .tree
                    .borrow_mut()
                    .pointer_raw_moved(*x, *y)
                    .expect("hover");
                ui.dispatch(&events);
                ui.settle();
            }
            Action::Press(x, y) => {
                let events = ui
                    .tree
                    .borrow_mut()
                    .pointer_raw_down(*x, *y)
                    .expect("press");
                ui.dispatch(&events);
                ui.settle();
            }
            Action::Release(x, y) => {
                let events = ui
                    .tree
                    .borrow_mut()
                    .pointer_raw_up(*x, *y)
                    .expect("release");
                ui.dispatch(&events);
                ui.settle();
            }
            Action::Leave => {
                let events = ui.tree.borrow_mut().pointer_raw_leave();
                ui.dispatch(&events);
                ui.settle();
            }
            Action::Key(code) => {
                let events = [RawUiEvent::window(RawEvent::key(
                    KeyPress::from(code.as_str()),
                    KeyModifiers::default(),
                ))];
                ui.dispatch_raw(&events);
                ui.settle();
            }
            Action::Advance(milliseconds) => {
                ui.tree
                    .borrow_mut()
                    .advance(Duration::from_millis(*milliseconds));
            }
            Action::Focus(id) => {
                ui.tree.borrow_mut().set_focused(Some(*id));
            }
            Action::Checked(id, checked) => {
                ui.tree.borrow_mut().set_checked(*id, *checked);
            }
            Action::Caret(id, fragment, offset) => {
                ui.tree
                    .borrow_mut()
                    .set_caret_position(*id, brul_gui::Position::new(*fragment, *offset));
            }
        }
    }

    display.finish(name, expected, expected_path);
}

macro_rules! display_cases {
    ($($name:ident),+ $(,)?) => {
        $(
            mod $name {
                include!(concat!(env!("OUT_DIR"), "/display_", stringify!($name), ".rs"));

                pub struct Case;

                impl super::Generated for Case {
                    type Theme = ThemeId;

                    fn parse_theme(value: &str) -> Result<Self::Theme, <Self::Theme as std::str::FromStr>::Err> {
                        value.parse()
                    }

                    fn windows() -> Vec<crate::WindowDef<Self::Theme>> {
                        windows()
                    }

                    fn names() -> &'static [&'static str] {
                        names()
                    }

                    fn script() -> Option<crate::Script> {
                        script()
                    }
                }
            }

            #[test]
            fn $name() {
                run_case::<$name::Case>(
                    stringify!($name),
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/display_brul/", stringify!($name), "/meta.txt")),
                    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/display_brul/", stringify!($name), "/expected.png")),
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/display_brul/", stringify!($name), "/expected.png"),
                );
            }
        )+
    };
}

display_cases! {
    button_dark,
    button_light,
    input_checkbox,
    card_dialog_tooltip,
}

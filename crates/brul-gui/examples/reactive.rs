use std::cell::RefCell;
use std::rc::Rc;

use brul_gui::window::{WindowConfig, run_tree};
use brul_gui::{
    Color, Dimension, LayoutStyle, Padding, StyleSet, StyleState, Widget, WidgetEvent, WidgetStyle,
    WidgetTree,
};
use brul_reactive::{Effect, Signal, batch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = WidgetStyle {
        background: Color::rgb(40, 44, 52),
        text: Color::rgb(240, 240, 240),
        radius: 12,
        ..WidgetStyle::default()
    };
    let active = WidgetStyle {
        background: Color::rgb(34, 197, 94),
        ..base
    };
    let checkbox = Widget::column(vec![Widget::text("Click me")])
        .with_layout(LayoutStyle {
            width: Dimension::Px(372),
            height: Dimension::Px(80),
            padding: Padding::all(20),
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(base).with(StyleState::Checked, active))
        .with_offset(24, 72);
    let checkbox_id = checkbox.id();
    let tree = Rc::new(RefCell::new(WidgetTree::new(checkbox)));
    let checked = Signal::new(false);
    let _effect = Effect::new({
        let tree = Rc::clone(&tree);
        move || {
            tree.borrow_mut().set_checked(checkbox_id, checked.get());
        }
    });

    let config = WindowConfig {
        title: String::from("BRUL reactive"),
        width: 420,
        height: 200,
        header_height: 64,
    };
    run_tree(config, tree, Vec::new(), move |events| {
        if events
            .iter()
            .any(|event| matches!(event, WidgetEvent::Click(_)))
        {
            batch(|| checked.set(!checked.get()));
        }
        Vec::new()
    })?;
    Ok(())
}

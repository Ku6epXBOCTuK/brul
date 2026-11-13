#![allow(clippy::non_ascii_literal)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use brul::gui::{Size, TextEngine, WidgetKind, WidgetSnapshot, WidgetTree};
use brul::{Component, ComponentHandle, Value};
use note_obs::generated;

fn seeded(presets: &[&str]) -> ComponentHandle {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut state = component.borrow_mut();
    state.set_names(generated::names());
    let presets = Value::Array(Rc::new(
        presets
            .iter()
            .map(|preset| Value::String((*preset).into()))
            .collect(),
    ));
    for (name, value) in [
        ("draft", Value::String("".into())),
        ("status", Value::String("idle".into())),
        ("live", Value::Bool(false)),
        (
            "hint",
            Value::String(
                "OBS obs · сцена Сцена · источник Источник · Ctrl+Enter — показать".into(),
            ),
        ),
        ("presets", presets),
        ("afkEnabled", Value::Bool(false)),
        ("afkText", Value::String("".into())),
        ("afkNote", Value::String("простой не отслеживается".into())),
    ] {
        state.declare(name, value, false).expect("seed");
    }
    drop(state);
    component
}

fn note_view(presets: &[&str]) -> WidgetTree {
    let built = generated::note_view(Some("dark"), &seeded(presets)).expect("generated view");
    let _ = brul::take_structures();
    WidgetTree::new(built.widget)
}

fn laid_out(tree: &mut WidgetTree) -> WidgetSnapshot {
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 520,
            height: 480,
        },
        1.0,
    )
    .expect("layout");
    tree.snapshot().expect("snapshot").clone()
}

fn measure(tree: &mut WidgetTree) -> (i64, u32) {
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 520,
            height: 480,
        },
        1.0,
    )
    .expect("layout");
    let snapshot = tree.snapshot().expect("snapshot");
    let bottom = snapshot
        .nodes
        .iter()
        .map(|node| i64::from(node.y) + i64::from(node.height))
        .max()
        .expect("nodes");
    let editor = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == WidgetKind::Input && node.width > 400)
        .map(|node| node.height)
        .expect("note field");
    (bottom, editor)
}

#[test]
fn the_note_fits_the_window_without_scrolling() {
    for presets in [&[][..], &["готово", "позже", "думаю", "вопрос"][..]] {
        let mut tree = note_view(presets);
        let (bottom, editor) = measure(&mut tree);
        assert!(
            bottom <= 480,
            "the content ends at {bottom} in a {}-preset window",
            presets.len()
        );
        assert!(
            editor >= 120,
            "the note field shrinks to {editor} in a {}-preset window",
            presets.len()
        );
    }
}

#[test]
fn the_preset_columns_line_up() {
    let mut tree = note_view(&["готово", "позже", "думаю", "вопрос"]);
    let snapshot = laid_out(&mut tree);
    let mut cells: Vec<(i32, i32, u32)> = snapshot
        .nodes
        .iter()
        .filter(|node| {
            node.kind == WidgetKind::View
                && node.height == 30
                && node.width > 200
                && node.width < 480
                && (230..320).contains(&node.y)
        })
        .map(|node| (node.y, node.x, node.width))
        .collect();
    cells.sort_unstable();
    cells.dedup();

    let mut rows: BTreeMap<i32, Vec<(i32, u32)>> = BTreeMap::new();
    for (y, x, width) in cells {
        rows.entry(y).or_default().push((x, width));
    }

    assert_eq!(
        rows.len(),
        2,
        "presets are not spread over two rows: {rows:?}"
    );
    for (y, row) in rows {
        assert_eq!(row.len(), 2, "row {y} has {} buttons", row.len());
        assert_eq!(row[0].1, row[1].1, "row {y} has buttons of different width");
        assert_eq!(
            row[1].0 - (row[0].0 + row[0].1 as i32),
            10,
            "row {y} has no gutter between the columns"
        );
    }
}

#[test]
fn the_note_window_opens_centered() {
    let windows = generated::windows();

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].name, "Note");
    assert_eq!(windows[0].config.title, "nobs");
    assert_eq!(windows[0].config.width, 520);
    assert_eq!(windows[0].config.height, 480);
    assert!(windows[0].config.centered);
    assert!(windows[0].config.visible);
}

#[test]
fn the_script_seeds_are_named() {
    let names = generated::names();

    for name in [
        "draft",
        "status",
        "hint",
        "presets",
        "afkEnabled",
        "afkText",
        "afkNote",
        "live",
    ] {
        assert!(names.contains(&name), "{name} is not declared");
    }
}

#[test]
fn the_afk_note_is_refreshed_by_a_timer() {
    let script = generated::script().expect("script");
    assert!(
        script
            .functions
            .iter()
            .any(|function| function.name == "tick"),
        "tick is not declared"
    );

    let scheduled = script.statements.iter().any(|statement| {
        let brul::script::Statement::Expr(expression) = statement else {
            return false;
        };
        let brul::ast::ExprKind::Call { target, args } = &expression.kind else {
            return false;
        };
        matches!(&target.kind, brul::ast::ExprKind::Ident(name) if name == "every")
            && matches!(args.get(1).map(|arg| &arg.kind), Some(brul::ast::ExprKind::Ident(name)) if name == "tick")
    });
    assert!(scheduled, "every is not called");
}

#[test]
fn the_afk_transitions_have_handlers() {
    let script = generated::script().expect("script");

    for name in ["afkAway", "afkBack"] {
        assert!(
            script
                .functions
                .iter()
                .any(|function| function.name == name),
            "{name} is not declared"
        );
    }
}

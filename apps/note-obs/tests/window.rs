use note_obs::generated;

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

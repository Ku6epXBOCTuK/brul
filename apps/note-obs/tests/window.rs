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

    for name in ["draft", "status", "hint", "presets"] {
        assert!(names.contains(&name), "{name} is not declared");
    }
}

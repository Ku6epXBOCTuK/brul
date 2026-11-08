use brul_syntax::markup::{Arg, Component, Declaration, File, Window, parse_file};

#[test]
fn component_with_empty_block_parses() {
    let file = parse_file("component CheckRow { }").unwrap();
    assert_eq!(
        file.declarations,
        vec![Declaration::Component(Component {
            name: "CheckRow".to_owned(),
        })]
    );
}

#[test]
fn window_parses_class_args() {
    let file = parse_file("window Picker (root, draggable) { }").unwrap();
    assert_eq!(
        file.declarations,
        vec![Declaration::Window(Window {
            name: "Picker".to_owned(),
            args: vec![
                Arg::Class("root".to_owned()),
                Arg::Class("draggable".to_owned())
            ],
        })]
    );
}

#[test]
fn declarations_and_comments_mix() {
    let source = "// header\ncomponent A { }\n\nwindow B (root) { } // tail\n";
    assert_eq!(parse_file(source).unwrap().declarations.len(), 2);
}

#[test]
fn empty_input_is_an_empty_file() {
    assert_eq!(
        parse_file("  // only a comment\n").unwrap(),
        File {
            declarations: Vec::new(),
        }
    );
}

#[test]
fn declaration_errors_point_at_the_source() {
    let error = parse_file("view X { }").unwrap_err();
    assert_eq!(error.message, "expected 'component' or 'window'");
    assert_eq!((error.line, error.column), (1, 1));

    let error = parse_file("component { }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 11));

    let error = parse_file("component A").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 12));

    let error = parse_file("component A { x }").unwrap_err();
    assert_eq!(error.message, "expected '}'");
    assert_eq!((error.line, error.column), (1, 15));

    let error = parse_file("window Picker () { }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 16));
}

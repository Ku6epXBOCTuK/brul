use brul_syntax::script::{Import, Program, parse};

#[test]
fn empty_script_is_an_empty_program() {
    assert_eq!(parse("").unwrap(), Program { imports: vec![] });
}

#[test]
fn import_parses_name_and_path() {
    let program = parse(r#"import BrowserItem from "./browser_item";"#).unwrap();
    assert_eq!(
        program.imports,
        vec![Import {
            name: "BrowserItem".to_owned(),
            path: "./browser_item".to_owned(),
        }]
    );
}

#[test]
fn imports_parse_in_order_and_skip_comments() {
    let source = r#"
        // components used by the markup
        import BrowserItem from "./browser_item";

        import Footer from "./footer";
    "#;
    let program = parse(source).unwrap();
    assert_eq!(
        program
            .imports
            .iter()
            .map(|import| import.name.as_str())
            .collect::<Vec<_>>(),
        vec!["BrowserItem", "Footer"]
    );
}

#[test]
fn statement_instead_of_import_is_rejected() {
    let error = parse("let browsers = [];").unwrap_err();
    assert_eq!(error.message, "expected 'import'");
    assert_eq!((error.line, error.column), (1, 1));
}

#[test]
fn import_without_name_points_at_the_path() {
    let error = parse(r#"import "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected import name");
    assert_eq!((error.line, error.column), (1, 8));
}

#[test]
fn import_without_from_points_at_the_path() {
    let error = parse(r#"import A "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected 'from'");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn import_with_non_string_path_is_rejected() {
    let error = parse("import A from x;").unwrap_err();
    assert_eq!(error.message, "expected module path");
    assert_eq!((error.line, error.column), (1, 15));
}

#[test]
fn import_without_semicolon_points_at_the_end() {
    let error = parse(r#"import A from "./a""#).unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 20));
}

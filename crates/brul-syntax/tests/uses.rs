use brul_syntax::script;
use brul_syntax::uses::{Resource, Use, parse};

#[test]
fn use_parses_name_and_path() {
    let block = parse(r#"import BrowserItem from "./browser_item";"#).unwrap();
    assert_eq!(
        block.uses,
        vec![Use {
            name: "BrowserItem".to_owned(),
            path: "./browser_item".to_owned(),
        }]
    );
    assert!(block.resources.is_empty());
}

#[test]
fn resource_parses_name_and_path() {
    let block = parse(r#"resource check from "./assets/check.png";"#).unwrap();
    assert_eq!(
        block.resources,
        vec![Resource {
            name: "check".to_owned(),
            path: "./assets/check.png".to_owned(),
        }]
    );
    assert!(block.uses.is_empty());
}

#[test]
fn use_items_parse_in_order_and_skip_comments() {
    let source = r#"
        // components used by the markup
        import BrowserItem from "./browser_item";

        import Footer from "./footer";
    "#;
    let block = parse(source).unwrap();
    assert_eq!(
        block
            .uses
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["BrowserItem", "Footer"]
    );
}

#[test]
fn imports_and_resources_parse_in_one_block() {
    let source = r#"
        import Badge from "./badge";
        resource check from "./check.png";
        resource cross from "./cross.png";
    "#;
    let block = parse(source).unwrap();
    assert_eq!(
        block
            .uses
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Badge"]
    );
    assert_eq!(
        block
            .resources
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["check", "cross"]
    );
}

#[test]
fn use_without_name_points_at_the_path() {
    let error = parse(r#"import "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected import name");
    assert_eq!((error.line, error.column), (1, 8));
}

#[test]
fn use_without_from_points_at_the_path() {
    let error = parse(r#"import A "./a";"#).unwrap_err();
    assert_eq!(error.message, "expected 'from'");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn use_with_non_string_path_is_rejected() {
    let error = parse("import A from x;").unwrap_err();
    assert_eq!(error.message, "expected import path");
    assert_eq!((error.line, error.column), (1, 15));
}

#[test]
fn use_without_semicolon_points_at_the_end() {
    let error = parse(r#"import A from "./a""#).unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 20));
}

#[test]
fn resource_without_name_points_at_the_path() {
    let error = parse(r#"resource "./a.png";"#).unwrap_err();
    assert_eq!(error.message, "expected resource name");
    assert_eq!((error.line, error.column), (1, 10));
}

#[test]
fn resource_without_from_points_at_the_path() {
    let error = parse(r#"resource A "./a.png";"#).unwrap_err();
    assert_eq!(error.message, "expected 'from'");
    assert_eq!((error.line, error.column), (1, 12));
}

#[test]
fn resource_with_non_string_path_is_rejected() {
    let error = parse("resource A from x;").unwrap_err();
    assert_eq!(error.message, "expected resource path");
    assert_eq!((error.line, error.column), (1, 17));
}

#[test]
fn resource_without_semicolon_points_at_the_end() {
    let error = parse(r#"resource A from "./a.png""#).unwrap_err();
    assert_eq!(error.message, "expected ';'");
    assert_eq!((error.line, error.column), (1, 26));
}

#[test]
fn empty_block_has_no_use_items() {
    let block = parse("").unwrap();
    assert!(block.uses.is_empty());
    assert!(block.resources.is_empty());
}

#[test]
fn script_block_does_not_parse_use_items() {
    assert!(script::parse(r#"import A from "./a";"#).is_err());
}

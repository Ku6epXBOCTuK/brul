use brul_syntax::script;
use brul_syntax::uses::{Use, parse};

#[test]
fn use_parses_name_and_path() {
    let uses = parse(r#"import BrowserItem from "./browser_item";"#).unwrap();
    assert_eq!(
        uses,
        vec![Use {
            name: "BrowserItem".to_owned(),
            path: "./browser_item".to_owned(),
        }]
    );
}

#[test]
fn use_items_parse_in_order_and_skip_comments() {
    let source = r#"
        // components used by the markup
        import BrowserItem from "./browser_item";

        import Footer from "./footer";
    "#;
    let uses = parse(source).unwrap();
    assert_eq!(
        uses.iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["BrowserItem", "Footer"]
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
fn empty_block_has_no_use_items() {
    assert!(parse("").unwrap().is_empty());
}

#[test]
fn script_block_does_not_parse_use_items() {
    assert!(script::parse(r#"import A from "./a";"#).is_err());
}

use super::*;

const MESSAGE: &str = "cannot resolve import 'x'";

#[test]
fn at_names_the_file_it_came_from() {
    let error = at(Some(Path::new("views/app.brul")), MESSAGE);
    assert!(error.message().contains("app.brul"));
    assert!(error.message().contains(MESSAGE));
}

#[test]
fn at_without_a_file_keeps_the_message_alone() {
    let error = at(None, MESSAGE);
    assert_eq!(error.message(), MESSAGE);
    assert_eq!((error.line(), error.column()), (1, 1));
}

#[test]
fn parse_rejects_a_file_without_a_declaration() {
    let source = r#"
<markup>
</markup>
"#;
    let error = parse(source, Path::new("empty.brul"), &[])
        .err()
        .expect("a file without a declaration must not parse");
    assert!(
        error.message().contains("file has no declaration"),
        "{}",
        error.message()
    );
    assert!(error.message().contains("empty.brul"));
}

#[test]
fn parse_keeps_the_uses_of_a_use_block() {
    let source = r#"
<use>
    import Card from "./card";
</use>

<markup>
component Screen {
    Text (text) { "hi" }
}
</markup>
"#;
    let parsed = parse(source, Path::new("screen.brul"), &[]).unwrap();
    assert_eq!(parsed.uses.uses.len(), 1);
    assert_eq!(parsed.uses.uses[0].name, "Card");
}

#[test]
fn a_resource_resolves_against_the_base_directory() {
    let dir = std::env::temp_dir().join(format!("brul-loader-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
            dir.join("check.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#,
        )
        .unwrap();
    let source = r#"
<use>
    resource Check from "./check.svg";
</use>

<markup>
component Card {
    Text (text) { "hi" }
}
</markup>
"#;
    let mut loader = Loader {
        definitions: Vec::new(),
        index: HashMap::new(),
        stack: Vec::new(),
        palette: &[],
    };
    let index = loader
        .define(
            Path::new("kit/card.brul").to_path_buf(),
            source.to_owned(),
            Some(&dir),
            Origin::Imported(Path::new("kit/card.brul").to_path_buf()),
        )
        .unwrap();
    let definition = &loader.definitions[index];
    assert_eq!(definition.resources.len(), 1);
    assert_eq!(definition.resources[0].name, "Check");
    assert_eq!(
        fs::read_to_string(&definition.resources[0].path).unwrap(),
        fs::read_to_string(dir.join("check.svg")).unwrap()
    );
    fs::remove_dir_all(&dir).unwrap();
}

use brul_lang_gen::types_doc;

const GENERATED: &str = include_str!("../../../docs/lang/types.md");

#[test]
fn types_doc_is_up_to_date() {
    assert_eq!(
        types_doc(),
        GENERATED,
        "types.md is out of date, run `cargo run -p brul-lang-gen --bin types-doc`"
    );
}

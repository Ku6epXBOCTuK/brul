use brul_lang_gen::builtins_doc;

const GENERATED: &str = include_str!("../../../docs/lang/builtins.md");

#[test]
fn builtins_doc_is_up_to_date() {
    assert_eq!(
        builtins_doc(),
        GENERATED,
        "builtins.md is out of date, run `cargo run -p brul-lang-gen --bin builtins-doc`"
    );
}

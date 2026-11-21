use brul_style_gen::properties_doc;

const GENERATED: &str = include_str!("../../../docs/lang/style_properties.md");

#[test]
fn properties_doc_is_up_to_date() {
    assert_eq!(
        properties_doc(),
        GENERATED,
        "style_properties.md is out of date, run `cargo run -p brul-style-gen --bin properties-doc`"
    );
}

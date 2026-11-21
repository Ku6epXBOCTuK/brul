use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("../../docs/lang/style_properties.md");
    fs::write(&path, brul_style_gen::properties_doc()).expect("cannot write style_properties.md");
}

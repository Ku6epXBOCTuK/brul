use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("../../docs/lang/types.md");
    fs::write(&path, brul_lang_gen::types_doc()).expect("cannot write types.md");
}

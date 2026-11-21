use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("../../docs/lang/builtins.md");
    fs::write(&path, brul_lang_gen::builtins_doc()).expect("cannot write builtins.md");
}

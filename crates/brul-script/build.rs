use std::path::Path;

fn main() {
    let out = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    brul_lang_gen::generate(brul_lang_gen::Output::ScriptCall, Path::new(&out));
}

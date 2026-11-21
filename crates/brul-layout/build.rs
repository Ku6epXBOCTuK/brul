use std::env;
use std::path::PathBuf;

fn main() {
    let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    brul_style_gen::generate(brul_style_gen::Output::Layout, &output);
}

use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    brul_style_gen::generate(brul_style_gen::Output::Style, &out_dir);
}

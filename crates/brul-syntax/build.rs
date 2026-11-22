use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    brul_style_gen::generate(brul_style_gen::Output::Syntax, &out_dir);
    brul_lang_gen::generate(brul_lang_gen::Output::Types, &out_dir);
    brul_lang_gen::generate(brul_lang_gen::Output::Builtins, &out_dir);
    brul_widget_gen::generate(&out_dir);
}

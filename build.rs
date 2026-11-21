use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    brul_lang_gen::generate(brul_lang_gen::Output::Markers, &out_dir);
    brul_lang_gen::generate(brul_lang_gen::Output::HostDispatch, &out_dir);
}

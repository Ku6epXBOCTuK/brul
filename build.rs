use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    brul_lang_gen::generate(brul_lang_gen::Output::Markers, &out_dir);
    brul_lang_gen::generate(brul_lang_gen::Output::HostDispatch, &out_dir);

    println!("cargo:rustc-check-cfg=cfg(brul_alloc_count)");
    let fixtures = root.join("tests/fixtures/display_brul");
    fs::write(
        out_dir.join("allocations.rs"),
        brul_codegen::emit::allocations(),
    )
    .expect("allocations");

    let mut cases: Vec<_> = fs::read_dir(&fixtures)
        .expect("display fixtures")
        .map(|entry| entry.expect("display fixture").path())
        .filter(|path| path.join("window.brul").is_file())
        .collect();
    cases.sort();
    for case in cases {
        let input = case.join("window.brul");
        println!("cargo:rerun-if-changed={}", input.display());
        let capabilities = brul_codegen::Capabilities::default();
        let module =
            brul_codegen::generate_with(&input, &capabilities).expect("display fixture generation");
        for source in &module.sources {
            println!("cargo:rerun-if-changed={}", source.display());
        }
        let name = case
            .file_name()
            .and_then(|name| name.to_str())
            .expect("display fixture name");
        fs::write(out_dir.join(format!("display_{name}.rs")), module.flat())
            .expect("display module");
    }
}

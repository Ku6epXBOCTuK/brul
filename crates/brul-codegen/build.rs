use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let dir = root.join("..").join("..").join("kit");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut sources = Vec::new();
    for entry in fs::read_dir(&dir).expect("cannot read the kit directory") {
        let path = entry.expect("cannot read a kit entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "brul")
        {
            sources.push(path);
        }
    }
    sources.sort();
    let dir = fs::canonicalize(&dir).expect("cannot canonicalize the kit directory");
    let mut table = String::from("pub static ROOT: &str = \"");
    table.push_str(&dir.display().to_string().replace('\\', "/"));
    table.push_str("\";\n\npub static SOURCES: &[(&str, &str)] = &[\n");
    for path in &sources {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a kit file name is not UTF-8");
        let literal = path.display().to_string().replace('\\', "/");
        writeln!(table, "    (\"{name}\", include_str!(r\"{literal}\")),")
            .expect("cannot write the kit table");
    }
    table.push_str("];\n");
    let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(output.join("kit.rs"), table).expect("cannot write the kit table");
    brul_style_gen::generate(brul_style_gen::Output::Codegen, &output);
    brul_style_gen::generate(brul_style_gen::Output::CodegenFields, &output);
}

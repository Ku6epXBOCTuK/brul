use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("bropicker build: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let source = root.join("src").join("window.brul");
    let module = brul::build::split_module_file(&source)?;
    let output = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(output.join("view.rs"), &module.root)?;
    for file in &module.files {
        fs::write(output.join(format!("{}.rs", file.name)), &file.source)?;
    }
    for file in sources(&root.join("src"))? {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    Ok(())
}

fn sources(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(sources(&path)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "brul")
        {
            found.push(path);
        }
    }
    Ok(found)
}

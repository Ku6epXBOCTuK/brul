//! Обвязка `build.rs`: генерация модуля интерфейса в `OUT_DIR`.

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use brul_syntax::error;

use crate::commands;
use crate::emit::split_module_file_with_commands;

const VIEW: &str = "view.rs";
const ALLOCATIONS: &str = "allocations.rs";
const RESOURCES: &str = "resources";

#[derive(thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Env(#[from] env::VarError),
    #[error(transparent)]
    Codegen(#[from] error::Error),
}

// NOTE: main() with Result prints the error with Debug, so it delegates to Display
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub fn app(entry: impl AsRef<Path>) -> Result<(), Error> {
    println!("cargo:rerun-if-env-changed=BRUL_ALLOC_COUNT");
    println!("cargo:rustc-check-cfg=cfg(brul_alloc_count)");
    if env::var_os("BRUL_ALLOC_COUNT").is_some_and(|value| value != "0" && !value.is_empty()) {
        println!("cargo:rustc-cfg=brul_alloc_count");
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let entry = root.join(entry);
    let commands = commands::collect(&root.join("src"))?;
    println!("cargo:rerun-if-changed={}", root.join("src").display());
    let module = split_module_file_with_commands(&entry, &commands)?;
    let output = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(output.join(VIEW), &module.root)?;
    fs::write(output.join(ALLOCATIONS), crate::emit::allocations())?;
    for file in &module.files {
        fs::write(output.join(format!("{}.rs", file.name)), &file.source)?;
    }
    for file in &module.scripts {
        fs::write(output.join(&file.name), &file.source)?;
    }
    if !module.resources.is_empty() {
        let resources = output.join(RESOURCES);
        fs::create_dir_all(&resources)?;
        for resource in &module.resources {
            fs::copy(&resource.path, resources.join(&resource.name))?;
            println!("cargo:rerun-if-changed={}", resource.path.display());
        }
    }
    for file in sources(entry.parent().unwrap_or(&root))? {
        println!("cargo:rerun-if-changed={}", file.display());
    }
    for file in &module.sources {
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

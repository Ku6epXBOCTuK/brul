//! Обвязка `build.rs`: генерация модуля интерфейса в `OUT_DIR`.

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use brul_syntax::error;

use crate::emit::split_module_file;

const VIEW: &str = "view.rs";

pub enum Error {
    Io(std::io::Error),
    Env(env::VarError),
    Codegen(error::Error),
}

// NOTE: main() with Result prints the error with Debug, so it delegates to Display
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Env(error) => write!(f, "{error}"),
            Self::Codegen(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<env::VarError> for Error {
    fn from(error: env::VarError) -> Self {
        Self::Env(error)
    }
}

impl From<error::Error> for Error {
    fn from(error: error::Error) -> Self {
        Self::Codegen(error)
    }
}

pub fn app(entry: impl AsRef<Path>) -> Result<(), Error> {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let entry = root.join(entry);
    let module = split_module_file(&entry)?;
    let output = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(output.join(VIEW), &module.root)?;
    for file in &module.files {
        fs::write(output.join(format!("{}.rs", file.name)), &file.source)?;
    }
    for file in &module.scripts {
        fs::write(output.join(&file.name), &file.source)?;
    }
    for file in sources(entry.parent().unwrap_or(&root))? {
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

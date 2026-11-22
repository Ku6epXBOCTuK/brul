//! CLI wrapper: argument parsing and output; generation lives in `generate`.

use std::env;
use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::emit::{self, Module};
use crate::{commands, generate, generate_with};

const INPUT: &str = "input.brul";
const OUTPUT: &str = "expected.rs";
const ERRORS: &str = "error.txt";
const COMMANDS: &str = "commands.txt";
const USAGE: &str = "usage: brul [--root <dir>] <input.brul> [--out <file.rs> | --dir <dir>] | brul --fixtures [dir]";

#[derive(thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Usage(String),
    #[error("cannot read '{path}': {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot read directory '{path}': {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write '{path}': {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write output: {source}")]
    Output {
        #[source]
        source: std::io::Error,
    },
    #[error("cannot remove '{path}': {source}")]
    Remove {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Codegen(#[from] brul_syntax::error::Error),
}

// NOTE: main() with Result prints the error with Debug, so it delegates to Display
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error {
    fn message(&self) -> String {
        match self {
            Self::Codegen(error) => error.message().to_owned(),
            other => other.to_string(),
        }
    }
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Error> {
    let mut args: Vec<String> = args.into_iter().collect();
    while args.first().is_some_and(|arg| arg == "brul" || arg == "--") {
        args.remove(0);
    }
    if let Some(index) = args.iter().position(|arg| arg == "--fixtures") {
        if index != 0 || args.len() > 2 {
            return Err(usage());
        }
        let dir = match args.get(1) {
            Some(path) => PathBuf::from(path),
            None => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codegen"),
        };
        return fixtures(&dir);
    }

    let mut root = None;
    let mut out = None;
    let mut dir = None;
    let mut input = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--out" => out = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            "--dir" => dir = Some(PathBuf::from(args.next().ok_or_else(usage)?)),
            other if other.starts_with("--") => return Err(usage()),
            other => {
                if input.replace(PathBuf::from(other)).is_some() {
                    return Err(usage());
                }
            }
        }
    }
    if out.is_some() && dir.is_some() {
        return Err(usage());
    }
    let input = input.ok_or_else(usage)?;
    let commands_dir = input
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let root = root.unwrap_or_else(|| commands_dir.to_path_buf());
    let module = source(&input, &root, &commands_dir.join(COMMANDS))?;
    match (out, dir) {
        (Some(path), None) => write(&path, module.flat()),
        (None, Some(path)) => write_split(&path, &module),
        (None, None) => write_stdout(module.flat()),
        _ => Err(usage()),
    }
}

fn source(input: &Path, root: &Path, commands_path: &Path) -> Result<Module, Error> {
    if commands_path.is_file() {
        generate_with(input, &commands::read(commands_path)?).map_err(Error::from)
    } else {
        generate(input, root).map_err(Error::from)
    }
}

fn write_split(dir: &Path, module: &Module) -> Result<(), Error> {
    write(&dir.join("view.rs"), &module.root)?;
    write(&dir.join("allocations.rs"), emit::allocations())?;
    for file in &module.files {
        write(&dir.join(format!("{}.rs", file.name)), &file.source)?;
    }
    for file in &module.scripts {
        write(&dir.join(&file.name), &file.source)?;
    }
    if !module.resources.is_empty() {
        let resources = dir.join("resources");
        fs::create_dir_all(&resources).map_err(|source| Error::Write {
            path: resources.clone(),
            source,
        })?;
        for resource in &module.resources {
            fs::copy(&resource.path, resources.join(&resource.name)).map_err(|source| {
                Error::Write {
                    path: resource.path.clone(),
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn write_stdout(source: &str) -> Result<(), Error> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(source.as_bytes())
        .map_err(|source| Error::Output { source })?;
    stdout.flush().map_err(|source| Error::Output { source })
}

fn fixtures(dir: &Path) -> Result<(), Error> {
    let entries = fs::read_dir(dir).map_err(|source| Error::ReadDir {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::ReadDir {
            path: dir.to_path_buf(),
            source,
        })?;
        let case = entry.path();
        let input = case.join(INPUT);
        if !case.is_dir() || !input.is_file() {
            continue;
        }
        let expected = case.join(OUTPUT);
        let errors = case.join(ERRORS);
        match source(&input, &case, &case.join(COMMANDS)).map(|module| module.flat().to_owned()) {
            Ok(text) => {
                write(&expected, text)?;
                remove(&errors)?;
            }
            Err(error) => {
                write(&errors, format!("{}\n", error.message()))?;
                remove(&expected)?;
            }
        }
    }
    Ok(())
}

fn usage() -> Error {
    Error::Usage(USAGE.to_owned())
}

fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<(), Error> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| Error::Write {
            path: path.to_path_buf(),
            source: error,
        })?;
    }
    fs::write(path, content).map_err(|error| Error::Write {
        path: path.to_path_buf(),
        source: error,
    })
}

fn remove(path: &Path) -> Result<(), Error> {
    if !path.is_file() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|source| Error::Remove {
        path: path.to_path_buf(),
        source,
    })
}

use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use brul_codegen::{Command, Commands, emit};
use brul_syntax::types::Type;

const INPUT: &str = "input.brul";
const OUTPUT: &str = "expected.rs";
const ERRORS: &str = "error.txt";
const COMMANDS: &str = "commands.txt";
const USAGE: &str = "usage: emit <input.brul> --out <file.rs> | emit --fixtures [dir]";

#[derive(thiserror::Error)]
enum Error {
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

fn main() -> Result<(), Error> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--fixtures") => {
            let dir = match args.next() {
                Some(path) => PathBuf::from(path),
                None => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codegen"),
            };
            if args.next().is_some() {
                return Err(Error::Usage(USAGE.to_owned()));
            }
            fixtures(&dir)
        }
        Some(input) => {
            let mut output = None;
            while let Some(flag) = args.next() {
                if flag != "--out" {
                    return Err(Error::Usage(USAGE.to_owned()));
                }
                output = Some(PathBuf::from(
                    args.next().ok_or_else(|| Error::Usage(USAGE.to_owned()))?,
                ));
            }
            let output = output.ok_or_else(|| Error::Usage(USAGE.to_owned()))?;
            let input = Path::new(input);
            let commands_dir = input
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            let source = generate(input, commands_dir)?;
            write(&output, source)
        }
        None => Err(Error::Usage(USAGE.to_owned())),
    }
}

fn generate(input: &Path, commands_dir: &Path) -> Result<String, Error> {
    let commands = load_commands(&commands_dir.join(COMMANDS))?;
    match emit::module_file_with_commands(input, &commands) {
        Ok(module) => Ok(module),
        Err(error) if error.message() == "missing window declaration" => {
            emit::component_file_with_commands(input, &commands).map_err(Error::from)
        }
        Err(error) => Err(Error::from(error)),
    }
}

fn load_commands(path: &Path) -> Result<Commands, Error> {
    let mut commands = Commands::new();
    if !path.is_file() {
        return Ok(commands);
    }
    for line in read(path)?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        let [name, address, params, result] = fields.as_slice() else {
            return Err(Error::Usage(format!("invalid command line '{line}'")));
        };
        let params = params
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(parse_type)
            .collect::<Result<Vec<_>, _>>()?;
        let command = Command {
            name: name.trim().to_owned(),
            path: address.trim().to_owned(),
            params,
            result: parse_type(result.trim())?,
        };
        commands.insert(command.name.clone(), command);
    }
    Ok(commands)
}

fn parse_type(name: &str) -> Result<Type, Error> {
    Type::from_name(name).ok_or_else(|| Error::Usage(format!("unknown command type '{name}'")))
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
        match generate(&input, &case) {
            Ok(source) => {
                write(&expected, source)?;
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

fn read(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })
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

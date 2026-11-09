use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Component as Declaration, Declaration as Item};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Stylesheet};
use brul_syntax::uses::{self, Use};

use crate::{error_at, padded, sfc_error};

pub struct Definition {
    pub name: String,
    pub path: PathBuf,
    pub component: Declaration,
    pub imports: Vec<Import>,
    pub stylesheet: Rc<Stylesheet>,
    pub program: Option<Program>,
}

#[derive(Clone)]
pub struct Import {
    pub name: String,
    pub definition: usize,
}

pub struct Registry {
    pub definitions: Vec<Definition>,
    pub imports: Vec<Import>,
}

pub fn resolve(uses: &[Use], base: Option<&Path>) -> Result<Registry, Error> {
    let mut loader = Loader {
        definitions: Vec::new(),
        index: HashMap::new(),
        stack: Vec::new(),
    };
    let imports = loader.imports(uses, base, None)?;
    Ok(Registry {
        definitions: loader.definitions,
        imports,
    })
}

struct Loader {
    definitions: Vec<Definition>,
    index: HashMap<PathBuf, usize>,
    stack: Vec<PathBuf>,
}

impl Loader {
    fn imports(
        &mut self,
        uses: &[Use],
        base: Option<&Path>,
        origin: Option<&Path>,
    ) -> Result<Vec<Import>, Error> {
        let mut imports = Vec::new();
        for item in uses {
            if imports
                .iter()
                .any(|existing: &Import| existing.name == item.name)
            {
                return Err(at(
                    origin,
                    format!("import '{}' is defined twice", item.name),
                ));
            }
            let path = file_path(base, &item.path, origin)?;
            let definition = self.file(&path, origin)?;
            imports.push(Import {
                name: item.name.clone(),
                definition,
            });
        }
        Ok(imports)
    }

    fn file(&mut self, path: &Path, origin: Option<&Path>) -> Result<usize, Error> {
        let path = fs::canonicalize(path)
            .map_err(|error| at(origin, format!("cannot read '{}': {error}", path.display())))?;
        if let Some(index) = self.index.get(&path) {
            return Ok(*index);
        }
        if self.stack.contains(&path) {
            return Err(self.cycle(&path));
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| at(origin, format!("cannot read '{}': {error}", path.display())))?;
        self.stack.push(path.clone());
        let parsed = parse(&source, &path)?;
        let imports = self.imports(&parsed.uses, path.parent(), Some(&path))?;
        self.stack.pop();
        let index = self.definitions.len();
        self.definitions.push(Definition {
            name: parsed.component.name.clone(),
            path: path.clone(),
            component: parsed.component,
            imports,
            stylesheet: parsed.stylesheet,
            program: parsed.program,
        });
        self.index.insert(path, index);
        Ok(index)
    }

    fn cycle(&self, path: &Path) -> Error {
        let mut chain: Vec<String> = self.stack.iter().map(|item| short(item)).collect();
        chain.push(short(path));
        error_at(
            Position { line: 1, column: 1 },
            format!("import cycle: {}", chain.join(" -> ")),
        )
    }
}

struct Parsed {
    component: Declaration,
    uses: Vec<Use>,
    stylesheet: Rc<Stylesheet>,
    program: Option<Program>,
}

fn parse(source: &str, path: &Path) -> Result<Parsed, Error> {
    let sfc = sfc::parse(source).map_err(|error| named(path, sfc_error(error)))?;
    let markup =
        markup::parse_file(&padded(source, sfc.markup)).map_err(|error| named(path, error))?;
    let component = markup
        .declarations
        .into_iter()
        .find_map(|item| match item {
            Item::Component(component) => Some(component),
            Item::Window(_) => None,
        })
        .ok_or_else(|| {
            named(
                path,
                error_at(
                    Position { line: 1, column: 1 },
                    "file has no component declaration",
                ),
            )
        })?;
    let uses = match sfc.uses {
        Some(block) => uses::parse(&padded(source, block)).map_err(|error| named(path, error))?,
        None => Vec::new(),
    };
    let stylesheet = match sfc.style {
        Some(block) => {
            style::parse_file(&padded(source, block)).map_err(|error| named(path, error))?
        }
        None => Stylesheet {
            themes: Vec::new(),
            rules: Vec::new(),
        },
    };
    let program = match sfc.script {
        Some(block) => {
            Some(script::parse(&padded(source, block)).map_err(|error| named(path, error))?)
        }
        None => None,
    };
    Ok(Parsed {
        component,
        uses,
        stylesheet: Rc::new(stylesheet),
        program,
    })
}

fn file_path(base: Option<&Path>, raw: &str, origin: Option<&Path>) -> Result<PathBuf, Error> {
    let Some(base) = base else {
        return Err(at(
            origin,
            format!("cannot resolve import '{raw}': source has no file path"),
        ));
    };
    let mut path = base.join(raw);
    if path.extension().is_none() {
        path.set_extension("brul");
    }
    Ok(path)
}

fn at(origin: Option<&Path>, message: impl Into<String>) -> Error {
    match origin {
        Some(path) => named(path, error_at(Position { line: 1, column: 1 }, message)),
        None => error_at(Position { line: 1, column: 1 }, message),
    }
}

fn named(path: &Path, mut error: Error) -> Error {
    error.message = format!("{}: {}", short(path), error.message);
    error
}

fn short(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

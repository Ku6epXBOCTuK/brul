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
use brul_syntax::uses::{self, Block, Use};

use crate::{error_at, padded, sfc_error};

pub struct Definition {
    pub name: String,
    pub path: PathBuf,
    pub component: Declaration,
    pub imports: Vec<Import>,
    pub resources: Vec<Resource>,
    pub stylesheet: Rc<Stylesheet>,
    pub program: Option<Program>,
}

#[derive(Clone)]
pub struct Import {
    pub name: String,
    pub definition: usize,
}

pub struct Resource {
    pub name: String,
    pub path: PathBuf,
}

pub struct Registry {
    pub definitions: Vec<Definition>,
    pub imports: Vec<Import>,
    pub resources: Vec<Resource>,
}

pub fn resolve(uses: &Block, base: Option<&Path>) -> Result<Registry, Error> {
    let mut loader = Loader {
        definitions: Vec::new(),
        index: HashMap::new(),
        stack: Vec::new(),
    };
    let imports = loader.imports(&uses.uses, base, None)?;
    let resources = loader.resources(&uses.resources, base, None)?;
    Ok(Registry {
        definitions: loader.definitions,
        imports,
        resources,
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

    fn resources(
        &self,
        resources: &[uses::Resource],
        base: Option<&Path>,
        origin: Option<&Path>,
    ) -> Result<Vec<Resource>, Error> {
        let mut resolved = Vec::new();
        for item in resources {
            if resolved
                .iter()
                .any(|existing: &Resource| existing.name == item.name)
            {
                return Err(at(
                    origin,
                    format!("resource '{}' is defined twice", item.name),
                ));
            }
            let path = resource_path(base, &item.path, origin)?;
            let path = fs::canonicalize(&path).map_err(|error| {
                at(
                    origin,
                    format!("cannot read resource '{}': {error}", item.path),
                )
            })?;
            let bytes = fs::read(&path).map_err(|error| {
                at(
                    origin,
                    format!("cannot read resource '{}': {error}", item.path),
                )
            })?;
            if !bytes.starts_with(&PNG) {
                return Err(at(origin, format!("resource '{}' is not a PNG", item.path)));
            }
            resolved.push(Resource {
                name: item.name.clone(),
                path,
            });
        }
        Ok(resolved)
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
        let imports = self.imports(&parsed.uses.uses, path.parent(), Some(&path))?;
        let resources = self.resources(&parsed.uses.resources, path.parent(), Some(&path))?;
        self.stack.pop();
        let index = self.definitions.len();
        self.definitions.push(Definition {
            name: parsed.component.name.clone(),
            path: path.clone(),
            component: parsed.component,
            imports,
            resources,
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
    uses: Block,
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
        None => Block::default(),
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

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn resource_path(base: Option<&Path>, raw: &str, origin: Option<&Path>) -> Result<PathBuf, Error> {
    let Some(base) = base else {
        return Err(at(
            origin,
            format!("cannot resolve resource '{raw}': source has no file path"),
        ));
    };
    Ok(base.join(raw))
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

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE: &str = "cannot resolve import 'x'";

    #[test]
    fn at_names_the_file_it_came_from() {
        let error = at(Some(Path::new("views/app.brul")), MESSAGE);
        assert!(error.message.contains("app.brul"));
        assert!(error.message.contains(MESSAGE));
    }

    #[test]
    fn at_without_a_file_keeps_the_message_alone() {
        let error = at(None, MESSAGE);
        assert_eq!(error.message, MESSAGE);
        assert_eq!((error.line, error.column), (1, 1));
    }

    #[test]
    fn parse_rejects_a_file_without_a_component() {
        let source = r#"
<markup>
window Picker (title: "x") {
    Column (root) { Text (text) { "hi" } }
}
</markup>
"#;
        let error = parse(source, Path::new("window.brul"))
            .err()
            .expect("a file without a component must not parse");
        assert!(
            error.message.contains("file has no component declaration"),
            "{}",
            error.message
        );
        assert!(error.message.contains("window.brul"));
    }

    #[test]
    fn parse_keeps_the_uses_of_a_use_block() {
        let source = r#"
<use>
    import Card from "./card";
</use>

<markup>
component Screen {
    Text (text) { "hi" }
}
</markup>
"#;
        let parsed = parse(source, Path::new("screen.brul")).unwrap();
        assert_eq!(parsed.uses.uses.len(), 1);
        assert_eq!(parsed.uses.uses[0].name, "Card");
    }
}

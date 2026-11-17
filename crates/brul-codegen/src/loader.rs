use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Component as Declaration, Declaration as Item};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Override, Stylesheet, Theme};
use brul_syntax::uses::{self, Block, Use};

use crate::source::Script;
use crate::{error_at, kit, padded, sfc_error};

pub struct Definition {
    pub name: String,
    pub path: PathBuf,
    pub component: Declaration,
    pub imports: Vec<Import>,
    pub resources: Vec<Resource>,
    pub stylesheet: Rc<Stylesheet>,
    pub program: Option<Program>,
    pub script: Option<Script>,
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

pub fn resolve(uses: &Block, base: Option<&Path>, palette: &[Theme]) -> Result<Registry, Error> {
    let mut loader = Loader {
        definitions: Vec::new(),
        index: HashMap::new(),
        stack: Vec::new(),
        palette,
    };
    let imports = loader.imports(&uses.uses, base, None)?;
    let resources = loader.resources(&uses.resources, base, None)?;
    Ok(Registry {
        definitions: loader.definitions,
        imports,
        resources,
    })
}

pub fn apply_overrides(
    registry: &mut Registry,
    overrides: &[Override],
    source: &str,
) -> Result<(), Error> {
    for over in overrides {
        let definition = registry
            .imports
            .iter()
            .find(|item| item.name == over.component)
            .map(|item| item.definition)
            .ok_or_else(|| {
                Error::at(
                    source,
                    over.component_at,
                    format!("override names '{}', which is not imported", over.component),
                )
            })?;
        let stylesheet = Rc::make_mut(&mut registry.definitions[definition].stylesheet);
        style::overrides(
            &over.component,
            &stylesheet.rules,
            &over.rules,
            &over.positions,
        )
        .map_err(|error| Error::at(source, error.at, error.message))?;
        stylesheet.rules.extend(over.rules.iter().cloned());
    }
    Ok(())
}

struct Loader<'a> {
    definitions: Vec<Definition>,
    index: HashMap<PathBuf, usize>,
    stack: Vec<PathBuf>,
    palette: &'a [Theme],
}

impl Loader<'_> {
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
            let definition = match item.path.strip_prefix(kit::PREFIX) {
                Some(name) => self.kit(name, origin)?,
                None => {
                    let path = file_path(base, &item.path, origin)?;
                    self.file(&path, origin)?
                }
            };
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
            if !bytes.starts_with(&PNG) && !is_svg(&bytes) {
                return Err(at(
                    origin,
                    format!("resource '{}' is neither PNG nor SVG", item.path),
                ));
            }
            resolved.push(Resource {
                name: item.name.clone(),
                path,
            });
        }
        Ok(resolved)
    }

    fn kit(&mut self, name: &str, origin: Option<&Path>) -> Result<usize, Error> {
        let Some(source) = kit::source(name) else {
            return Err(at(
                origin,
                format!(
                    "unknown kit component '{name}': the kit has {}",
                    kit::names().join(", ")
                ),
            ));
        };
        let path = kit::path(name);
        if let Some(index) = self.index.get(&path) {
            return Ok(*index);
        }
        if self.stack.contains(&path) {
            return Err(self.cycle(&path));
        }
        self.define(path, source.to_owned(), Some(kit::dir()))
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
        let base = path.parent().map(Path::to_path_buf);
        self.define(path, source, base.as_deref())
    }

    fn define(
        &mut self,
        path: PathBuf,
        source: String,
        base: Option<&Path>,
    ) -> Result<usize, Error> {
        self.stack.push(path.clone());
        let parsed = parse(&source, &path, self.palette)?;
        let imports = self.imports(&parsed.uses.uses, base, Some(&path))?;
        let resources = self.resources(&parsed.uses.resources, base, Some(&path))?;
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
            script: parsed.script,
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
    script: Option<Script>,
}

fn parse(source: &str, path: &Path, palette: &[Theme]) -> Result<Parsed, Error> {
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
        Some(block) => style::parse_file_in(&padded(source, block), palette)
            .map_err(|error| named(path, error))?,
        None => Stylesheet {
            themes: palette.to_vec(),
            rules: Vec::new(),
        },
    };
    let (program, script) = match sfc.script {
        Some(block) => (
            Some(script::parse(&padded(source, block)).map_err(|error| named(path, error))?),
            Some(Script::new(source, block)),
        ),
        None => (None, None),
    };
    Ok(Parsed {
        component,
        uses,
        stylesheet: Rc::new(stylesheet),
        program,
        script,
    })
}

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn is_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    head.trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('<')
}

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
        let error = parse(source, Path::new("window.brul"), &[])
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
        let parsed = parse(source, Path::new("screen.brul"), &[]).unwrap();
        assert_eq!(parsed.uses.uses.len(), 1);
        assert_eq!(parsed.uses.uses[0].name, "Card");
    }

    #[test]
    fn a_resource_resolves_against_the_base_directory() {
        let dir = std::env::temp_dir().join(format!("brul-loader-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("check.svg"),
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8"/></svg>"#,
        )
        .unwrap();
        let source = r#"
<use>
    resource Check from "./check.svg";
</use>

<markup>
component Card {
    Text (text) { "hi" }
}
</markup>
"#;
        let mut loader = Loader {
            definitions: Vec::new(),
            index: HashMap::new(),
            stack: Vec::new(),
            palette: &[],
        };
        let index = loader
            .define(
                Path::new("kit/card.brul").to_path_buf(),
                source.to_owned(),
                Some(&dir),
            )
            .unwrap();
        let definition = &loader.definitions[index];
        assert_eq!(definition.resources.len(), 1);
        assert_eq!(definition.resources[0].name, "Check");
        assert_eq!(
            fs::read_to_string(&definition.resources[0].path).unwrap(),
            fs::read_to_string(dir.join("check.svg")).unwrap()
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}

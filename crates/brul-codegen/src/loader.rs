use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Declaration};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, EffectRule, Override, Stylesheet, Theme};
use brul_syntax::uses::{self, Block, Use};

use crate::source::Script;
use crate::{error_at, kit, padded, sfc_error};

pub struct Definition {
    pub name: String,
    pub file: String,
    pub declaration: Declaration,
    pub origin: Origin,
    pub imports: Vec<Import>,
    pub resources: Vec<Resource>,
    pub stylesheet: Rc<Stylesheet>,
    pub effects: Vec<EffectRule>,
    pub program: Option<Program>,
    pub script: Option<Script>,
}

impl Definition {
    pub fn is_window(&self) -> bool {
        self.declaration.is_window()
    }
}

pub enum Origin {
    Entry,
    Imported(PathBuf),
    Kit,
}

impl Origin {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Imported(path) => Some(path),
            Self::Entry | Self::Kit => None,
        }
    }
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

pub fn load_entry(path: &Path) -> Result<(Registry, usize), Error> {
    let mut loader = Loader {
        definitions: Vec::new(),
        index: HashMap::new(),
        stack: Vec::new(),
        palette: &[],
    };
    let path = fs::canonicalize(path).map_err(|error| {
        at(
            Some(path),
            format!("cannot read '{}': {error}", path.display()),
        )
    })?;
    let index = loader.file(&path, Some(&path))?;
    loader.definitions[index].origin = Origin::Entry;
    let imports = loader.definitions[index].imports.clone();
    let registry = Registry {
        definitions: loader.definitions,
        imports,
        resources: Vec::new(),
    };
    Ok((registry, index))
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
                    Error::OverrideNotImported(over.component.clone()),
                )
            })?;
        let stylesheet = Rc::make_mut(&mut registry.definitions[definition].stylesheet);
        style::overrides(
            &over.component,
            &stylesheet.rules,
            &over.rules,
            &over.positions,
        )
        .map_err(|error| Error::at(source, error.at, Error::custom(error.message)))?;
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
        self.define(path, source.to_owned(), Some(kit::dir()), Origin::Kit)
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
        self.define(
            path.clone(),
            source,
            base.as_deref(),
            Origin::Imported(path),
        )
    }

    fn define(
        &mut self,
        path: PathBuf,
        source: String,
        base: Option<&Path>,
        origin: Origin,
    ) -> Result<usize, Error> {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown.brul".to_owned());
        self.stack.push(path.clone());
        let parsed = parse(&source, &path, self.palette)?;
        let imports = self.imports(&parsed.uses.uses, base, Some(&path))?;
        let resources = self.resources(&parsed.uses.resources, base, Some(&path))?;
        self.stack.pop();
        let index = self.definitions.len();
        self.definitions.push(Definition {
            name: parsed.declaration.name.clone(),
            file,
            declaration: parsed.declaration,
            origin,
            imports,
            resources,
            stylesheet: parsed.stylesheet,
            effects: parsed.effects,
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
    declaration: Declaration,
    uses: Block,
    stylesheet: Rc<Stylesheet>,
    effects: Vec<EffectRule>,
    program: Option<Program>,
    script: Option<Script>,
}

fn parse(source: &str, path: &Path, palette: &[Theme]) -> Result<Parsed, Error> {
    let sfc = sfc::parse(source).map_err(|error| named(path, sfc_error(error)))?;
    let markup =
        markup::parse_file(&padded(source, sfc.markup)).map_err(|error| named(path, error))?;
    let declaration = markup.declaration.ok_or_else(|| {
        named(
            path,
            error_at(Position { line: 1, column: 1 }, "file has no declaration"),
        )
    })?;
    let uses = match sfc.uses {
        Some(block) => uses::parse(&padded(source, block)).map_err(|error| named(path, error))?,
        None => Block::default(),
    };
    let (stylesheet, effects) = match sfc.style {
        Some(block) => style::parse_file_in_with_effects(&padded(source, block), palette)
            .map_err(|error| named(path, error))?,
        None => (
            Stylesheet {
                themes: palette.to_vec(),
                rules: Vec::new(),
            },
            Vec::new(),
        ),
    };
    let (program, script) = match sfc.script {
        Some(block) => (
            Some(script::parse(&padded(source, block)).map_err(|error| named(path, error))?),
            Some(Script::new(source, block)),
        ),
        None => (None, None),
    };
    Ok(Parsed {
        declaration,
        uses,
        stylesheet: Rc::new(stylesheet),
        effects,
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

fn named(path: &Path, error: Error) -> Error {
    error.named(short(path))
}

fn short(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "internal/loader_tests.rs"]
mod tests;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Declaration, Window};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Stylesheet};
use brul_syntax::uses::{self, Block};

use crate::{error_at, padded, sfc_error};

pub struct Source {
    pub windows: Vec<Window>,
    pub stylesheet: Stylesheet,
    pub uses: Block,
    pub program: Option<Program>,
    pub script: Option<Script>,
}

pub struct Script {
    pub text: String,
    pub line: usize,
}

impl Script {
    pub fn new(source: &str, block: sfc::Block<'_>) -> Self {
        Self {
            text: block.content.to_owned(),
            line: source[..block.offset].matches('\n').count() + 1,
        }
    }
}

pub fn parse(source: &str) -> Result<Source, Error> {
    let sfc = sfc::parse(source).map_err(sfc_error)?;
    let markup = markup::parse_file(&padded(source, sfc.markup))?;
    let mut windows = Vec::new();
    for declaration in markup.declarations {
        if let Declaration::Window(window) = declaration {
            if windows.iter().any(|seen: &Window| seen.name == window.name) {
                return Err(error_at(
                    window.position,
                    format!("duplicate window '{}'", window.name),
                ));
            }
            windows.push(window);
        }
    }
    if windows.is_empty() {
        return Err(error_at(
            Position { line: 1, column: 1 },
            "missing window declaration",
        ));
    }
    let stylesheet = match sfc.style {
        Some(block) => style::parse_file(&padded(source, block))?,
        None => Stylesheet {
            themes: Vec::new(),
            rules: Vec::new(),
        },
    };
    let uses = match sfc.uses {
        Some(block) => uses::parse(&padded(source, block))?,
        None => Block::default(),
    };
    let (program, script) = match sfc.script {
        Some(block) => (
            Some(script::parse(&padded(source, block))?),
            Some(Script::new(source, block)),
        ),
        None => (None, None),
    };
    Ok(Source {
        windows,
        stylesheet,
        uses,
        program,
        script,
    })
}

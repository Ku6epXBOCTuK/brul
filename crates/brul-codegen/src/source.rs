use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Declaration};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Override, Stylesheet};
use brul_syntax::uses::{self, Block};

use crate::{error_at, padded, sfc_error};

pub struct Source {
    pub declaration: Option<Declaration>,
    pub stylesheet: Stylesheet,
    pub overrides: Vec<Override>,
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

impl Source {
    pub fn empty() -> Self {
        Self {
            declaration: None,
            stylesheet: Stylesheet {
                themes: Vec::new(),
                rules: Vec::new(),
            },
            overrides: Vec::new(),
            uses: Block::default(),
            program: None,
            script: None,
        }
    }
}

pub fn parse(source: &str) -> Result<Source, Error> {
    let sfc = sfc::parse(source).map_err(sfc_error)?;
    let markup = markup::parse_file(&padded(source, sfc.markup))?;
    let Some(declaration) = markup.declaration else {
        return Err(error_at(
            Position { line: 1, column: 1 },
            "missing declaration",
        ));
    };
    let style = match sfc.style {
        Some(block) => style::parse_root(&padded(source, block))?,
        None => style::Style {
            stylesheet: Stylesheet {
                themes: Vec::new(),
                rules: Vec::new(),
            },
            overrides: Vec::new(),
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
        declaration: Some(declaration),
        stylesheet: style.stylesheet,
        overrides: style.overrides,
        uses,
        program,
        script,
    })
}

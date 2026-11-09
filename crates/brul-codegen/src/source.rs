use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::markup::{self, Declaration, Window};
use brul_syntax::script::{self, Program};
use brul_syntax::sfc;
use brul_syntax::style::{self, Stylesheet};
use brul_syntax::uses::{self, Use};

use crate::{error_at, padded, sfc_error};

pub struct Source {
    pub window: Window,
    pub stylesheet: Stylesheet,
    pub uses: Vec<Use>,
    pub program: Option<Program>,
}

pub fn parse(source: &str) -> Result<Source, Error> {
    let sfc = sfc::parse(source).map_err(sfc_error)?;
    let markup = markup::parse_file(&padded(source, sfc.markup))?;
    let window = markup
        .declarations
        .into_iter()
        .find_map(|declaration| match declaration {
            Declaration::Window(window) => Some(window),
            Declaration::Component(_) => None,
        })
        .ok_or_else(|| {
            error_at(
                Position { line: 1, column: 1 },
                "missing window declaration",
            )
        })?;
    let stylesheet = match sfc.style {
        Some(block) => style::parse_file(&padded(source, block))?,
        None => Stylesheet {
            themes: Vec::new(),
            rules: Vec::new(),
        },
    };
    let uses = match sfc.uses {
        Some(block) => uses::parse(&padded(source, block))?,
        None => Vec::new(),
    };
    let program = match sfc.script {
        Some(block) => Some(script::parse(&padded(source, block))?),
        None => None,
    };
    Ok(Source {
        window,
        stylesheet,
        uses,
        program,
    })
}

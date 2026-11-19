use pest::Parser;
use pest::error::LineColLocation;
use pest_derive::Parser;

use crate::ast::Position;

#[derive(Parser)]
#[grammar = "sfc.pest"]
struct SfcParser;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block<'a> {
    pub content: &'a str,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sfc<'a> {
    pub uses: Option<Block<'a>>,
    pub markup: Block<'a>,
    pub style: Option<Block<'a>>,
    pub script: Option<Block<'a>>,
}

pub use crate::error::Error;

pub fn parse(source: &str) -> Result<Sfc<'_>, Error> {
    let pairs = SfcParser::parse(Rule::file, source).map_err(|error| {
        let (line, column) = match error.line_col {
            LineColLocation::Pos(pos) | LineColLocation::Span(pos, _) => pos,
        };
        Error::at_position(
            Position { line, column },
            Error::custom(error.variant.message()),
        )
    })?;

    let mut uses = None;
    let mut markup = None;
    let mut style = None;
    let mut script = None;

    for pair in pairs.into_iter().flat_map(|file| file.into_inner()) {
        let (line, column) = pair.as_span().start_pos().line_col();
        let name = match pair.as_rule() {
            Rule::use_block => "use",
            Rule::markup => "markup",
            Rule::style => "style",
            Rule::script => "script",
            _ => continue,
        };
        let slot = match name {
            "use" => &mut uses,
            "markup" => &mut markup,
            "style" => &mut style,
            _ => &mut script,
        };
        if slot.replace(block(pair, source)).is_some() {
            return Err(Error::at_position(
                Position { line, column },
                Error::DuplicateBlock(name),
            ));
        }
    }

    Ok(Sfc {
        uses,
        markup: markup.ok_or_else(|| {
            Error::at_position(Position { line: 1, column: 1 }, Error::MissingMarkup)
        })?,
        style,
        script,
    })
}

fn block<'a>(pair: pest::iterators::Pair<'a, Rule>, source: &'a str) -> Block<'a> {
    let inner: Vec<_> = pair.into_inner().collect();
    let start = inner.first().expect("open tag").as_span().end();
    let end = inner.last().expect("close tag").as_span().start();
    Block {
        content: &source[start..end],
        offset: start,
    }
}

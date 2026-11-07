use std::fmt;

use pest::Parser;
use pest::error::LineColLocation;
use pest_derive::Parser;

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
    pub markup: Block<'a>,
    pub style: Option<Block<'a>>,
    pub script: Option<Block<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Syntax {
        message: String,
        line: usize,
        column: usize,
    },
    Duplicate {
        name: &'static str,
        line: usize,
        column: usize,
    },
    MissingMarkup,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                message,
                line,
                column,
            } => write!(f, "строка {line}, колонка {column}: {message}"),
            Self::Duplicate { name, line, column } => {
                write!(
                    f,
                    "строка {line}, колонка {column}: повторный блок <{name}>"
                )
            }
            Self::MissingMarkup => write!(f, "нет блока <markup>"),
        }
    }
}

impl std::error::Error for Error {}

pub fn parse(source: &str) -> Result<Sfc<'_>, Error> {
    let pairs = SfcParser::parse(Rule::file, source).map_err(|error| {
        let (line, column) = match error.line_col {
            LineColLocation::Pos(pos) | LineColLocation::Span(pos, _) => pos,
        };
        Error::Syntax {
            message: error.variant.message().to_string(),
            line,
            column,
        }
    })?;

    let mut markup = None;
    let mut style = None;
    let mut script = None;

    for pair in pairs.into_iter().flat_map(|file| file.into_inner()) {
        let (line, column) = pair.as_span().start_pos().line_col();
        let name = match pair.as_rule() {
            Rule::markup => "markup",
            Rule::style => "style",
            Rule::script => "script",
            _ => continue,
        };
        let slot = match name {
            "markup" => &mut markup,
            "style" => &mut style,
            _ => &mut script,
        };
        if slot.replace(block(pair, source)).is_some() {
            return Err(Error::Duplicate { name, line, column });
        }
    }

    Ok(Sfc {
        markup: markup.ok_or(Error::MissingMarkup)?,
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

use std::fmt;

use crate::ast::Position;
use crate::lexer::{Punct, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unterminated {
    String,
    Template,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expected {
    Punct(Punct),
    Char(char),
    Keyword(&'static str),
    Word(&'static str),
    OneOf(&'static str),
    AtDirective(&'static str),
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Punct(punct) => write!(f, "'{punct}'"),
            Self::Char(ch) => write!(f, "'{ch}'"),
            Self::Keyword(keyword) => write!(f, "'{keyword}'"),
            Self::Word(word) => f.write_str(word),
            Self::OneOf(options) => f.write_str(options),
            Self::AtDirective(name) => write!(f, "'@{name}'"),
        }
    }
}

impl From<&'static str> for Expected {
    fn from(word: &'static str) -> Self {
        Self::Word(word)
    }
}

impl From<Punct> for Expected {
    fn from(punct: Punct) -> Self {
        Self::Punct(punct)
    }
}

impl fmt::Display for Unterminated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::String => "string",
            Self::Template => "template",
        })
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("{0}: {1}")]
    At(Position, #[source] Box<Error>),

    #[error("expected {0}")]
    Expected(Expected),
    #[error("expected {expected} for '{name}'")]
    ExpectedFor {
        expected: &'static str,
        name: String,
    },
    #[error("expected number from {min} to {max} for '{name}'")]
    NumberRange { min: f64, max: f64, name: String },
    #[error("expected {options} for '{name}'")]
    OneOf { options: String, name: String },
    #[error("expected percent from 0 to 100 for '{0}'")]
    PercentRange(String),
    #[error("expected color or px for theme key '{0}'")]
    ExpectedThemeKey(String),
    #[error("expected at most {max} values for '{name}'")]
    AtMost { max: usize, name: String },
    #[error("expected 3, 6 or 8 hex digits")]
    HexDigits,

    #[error("unterminated {0}")]
    Unterminated(Unterminated),
    #[error("unknown character {0:?}")]
    UnknownCharacter(char),
    #[error("expected '{{' after '$'")]
    DollarBrace,

    #[error("{0} '{1}' is not defined")]
    Undefined(String, String),
    #[error("unknown property '{0}'")]
    UnknownProperty(String),
    #[error("unknown state")]
    UnknownState,
    #[error("unknown theme key '{0}'")]
    UnknownThemeKey(String),
    #[error("unexpected directive '@{0}'")]
    UnexpectedDirective(String),
    #[error("expected '@{0}': block constructs start with '@'")]
    ExpectedAtDirective(String),
    #[error("unexpected token")]
    UnexpectedToken,

    #[error("duplicate <{0}> block")]
    DuplicateBlock(&'static str),
    #[error("missing <markup> block")]
    MissingMarkup,
    #[error("duplicate window '{0}'")]
    DuplicateWindow(String),
    #[error("a file can contain only one declaration")]
    MultipleDeclarations,
    #[error("missing window declaration")]
    MissingWindow,
    #[error("override names '{0}', which is not imported")]
    OverrideNotImported(String),
    #[error("import cycle: {0}")]
    ImportCycle(String),
    #[error("file has no component declaration")]
    NoComponent,

    #[error("{what} is nested deeper than {max}")]
    NestingDepth { what: &'static str, max: usize },
    #[error("theme key '{0}' cannot alias another key; use it in an expression")]
    ThemeAlias(String),
    #[error("theme key '{key}' is a {actual}, but '{name}' expects a {expected}")]
    ThemeKeyType {
        key: String,
        name: String,
        actual: &'static str,
        expected: &'static str,
    },
    #[error("property '{0}' cannot be animated")]
    NotAnimatable(String),
    #[error("a theme can only be declared in the root <style>")]
    ThemeNotRoot,
    #[error("an override can only be declared in the root <style>")]
    OverrideNotRoot,
    #[error("functions are only allowed at the top level")]
    FunctionNotTopLevel,
    #[error("return is only allowed inside a function")]
    ReturnOutsideFunction,
    #[error("const requires an initializer")]
    ConstNoInitializer,
    #[error("{0} is a leaf widget: nested elements are not allowed")]
    LeafWidget(String),

    #[error("{path}: {source}")]
    Named {
        path: String,
        #[source]
        source: Box<Error>,
    },
    #[error("{0}")]
    Message(String),
}

impl From<Expected> for Error {
    fn from(expected: Expected) -> Self {
        Self::Expected(expected)
    }
}

impl From<Unterminated> for Error {
    fn from(kind: Unterminated) -> Self {
        Self::Unterminated(kind)
    }
}

impl Error {
    pub fn at(source: &str, pos: usize, reason: impl Into<Self>) -> Self {
        Self::at_span(
            source,
            Span {
                start: pos,
                end: pos,
            },
            reason,
        )
    }

    pub fn at_span(source: &str, span: Span, reason: impl Into<Self>) -> Self {
        Self::At(span.position(source), Box::new(reason.into()))
    }

    pub fn at_position(position: Position, reason: impl Into<Self>) -> Self {
        Self::At(position, Box::new(reason.into()))
    }

    pub fn custom(reason: impl Into<String>) -> Self {
        Self::Message(reason.into())
    }

    pub fn position(&self) -> Option<Position> {
        match self {
            Self::At(position, _) => Some(*position),
            Self::Named { source, .. } => source.position(),
            _ => None,
        }
    }

    pub fn line(&self) -> usize {
        self.position().unwrap_or_default().line
    }

    pub fn column(&self) -> usize {
        self.position().unwrap_or_default().column
    }

    pub fn message(&self) -> String {
        match self {
            Self::At(_, error) => error.message(),
            Self::Named { path, source } => format!("{path}: {}", source.message()),
            error => error.to_string(),
        }
    }

    pub fn named(self, path: impl Into<String>) -> Self {
        Self::Named {
            path: path.into(),
            source: Box::new(self),
        }
    }
}

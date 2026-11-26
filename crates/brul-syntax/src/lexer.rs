#[path = "lexer/scan.rs"]
mod scan;
use scan::{TemplateStep, digits_end, error, punct_pos, template_step};
use std::collections::VecDeque;
use std::fmt;

use crate::ast::Position;
use crate::error::{Error as ParseError, Expected, Unterminated};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn line_col(self, source: &str) -> (usize, usize) {
        let before = &source[..self.start];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = before
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
        (line, column)
    }

    pub fn position(self, source: &str) -> Position {
        let (line, column) = self.line_col(source);
        Position { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punct {
    Assign,
    PlusAssign,
    MinusAssign,
    Question,
    Colon,
    Or,
    And,
    Eq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Dot,
    Comma,
    Semicolon,
    At,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
}

impl fmt::Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Assign => "=",
            Self::PlusAssign => "+=",
            Self::MinusAssign => "-=",
            Self::Question => "?",
            Self::Colon => ":",
            Self::Or => "||",
            Self::And => "&&",
            Self::Eq => "==",
            Self::NotEq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Bang => "!",
            Self::Dot => ".",
            Self::Comma => ",",
            Self::Semicolon => ";",
            Self::At => "@",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::LBracket => "[",
            Self::RBracket => "]",
            Self::LBrace => "{",
            Self::RBrace => "}",
        };
        f.write_str(text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Number(f64),
    String(String),
    TemplateStart,
    TemplateChunk(String),
    TemplateInterpStart,
    TemplateInterpEnd,
    TemplateEnd,
    Punct(Punct),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct TokenStream<'a> {
    pub(crate) source: &'a str,
    tokens: Vec<Token>,
    at: usize,
    lexer: Option<Lexer<'a>>,
}

impl<'a> TokenStream<'a> {
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            at: 0,
            lexer: None,
        }
    }

    pub(crate) fn lazy(source: &'a str, lexer: Lexer<'a>) -> Self {
        Self {
            source,
            tokens: Vec::new(),
            at: 0,
            lexer: Some(lexer),
        }
    }

    pub(crate) fn peek(&mut self) -> Result<&Token, ParseError> {
        while self.tokens.len() <= self.at {
            let Some(lexer) = self.lexer.as_mut() else {
                break;
            };
            match lexer.next() {
                Some(Ok(token)) => self.tokens.push(token),
                Some(Err(error)) => return Err(error),
                None => break,
            }
        }
        Ok(&self.tokens[self.at])
    }

    pub(crate) fn bump(&mut self) {
        self.at += 1;
    }

    pub(crate) fn last_end(&self) -> usize {
        self.tokens[self.at - 1].span.end
    }

    pub fn at_end(&mut self) -> Result<bool, ParseError> {
        Ok(self.peek()?.kind == TokenKind::Eof)
    }

    pub fn at_keyword(&mut self, keyword: &str) -> Result<bool, ParseError> {
        Ok(matches!(&self.peek()?.kind, TokenKind::Ident(name) if name == keyword))
    }

    pub fn expect_keyword(&mut self, keyword: &'static str) -> Result<(), ParseError> {
        let token = self.peek()?;
        let span = token.span;
        let matches = matches!(&token.kind, TokenKind::Ident(name) if name == keyword);
        if matches {
            self.bump();
            Ok(())
        } else {
            Err(self.error(span, Expected::Keyword(keyword)))
        }
    }

    pub fn ident(&mut self, expected: impl Into<Expected>) -> Result<String, ParseError> {
        let token = self.peek()?;
        let span = token.span;
        match token.kind.clone() {
            TokenKind::Ident(name) => {
                self.bump();
                Ok(name)
            }
            _ => Err(self.error(span, expected.into())),
        }
    }

    pub fn string(&mut self, expected: impl Into<Expected>) -> Result<String, ParseError> {
        let token = self.peek()?;
        let span = token.span;
        match token.kind.clone() {
            TokenKind::String(value) => {
                self.bump();
                Ok(value)
            }
            _ => Err(self.error(span, expected.into())),
        }
    }

    pub fn expect_punct(
        &mut self,
        punct: Punct,
        expected: impl Into<Expected>,
    ) -> Result<(), ParseError> {
        let token = self.peek()?;
        let span = token.span;
        let matches = token.kind == TokenKind::Punct(punct);
        if matches {
            self.bump();
            Ok(())
        } else {
            Err(self.error(span, expected.into()))
        }
    }

    pub fn error(&self, span: Span, reason: impl Into<ParseError>) -> ParseError {
        ParseError::at_span(self.source, span, reason)
    }
}

#[derive(Clone, Copy)]
enum Frame {
    Code { braces: usize, interpolation: bool },
    Template { start: usize },
}

pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    frames: Vec<Frame>,
    pending: VecDeque<Token>,
    finished: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, offset: usize) -> Self {
        Self {
            source,
            pos: offset,
            frames: vec![Frame::Code {
                braces: 0,
                interpolation: false,
            }],
            pending: VecDeque::new(),
            finished: false,
        }
    }

    fn step(&mut self) -> Result<(), ParseError> {
        while !self.finished && self.pending.is_empty() {
            if self.pos >= self.source.len() {
                self.finish()?;
                break;
            }
            self.pos = match *self.frames.last().expect("code frame is always present") {
                Frame::Template { start } => template_pos(
                    self.source,
                    self.pos,
                    start,
                    &mut self.pending,
                    &mut self.frames,
                )?,
                Frame::Code { .. } => {
                    code_pos(self.source, self.pos, &mut self.pending, &mut self.frames)?
                }
            };
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), ParseError> {
        self.finished = true;
        if let Some(start) = self.frames.iter().find_map(|frame| match frame {
            Frame::Template { start } => Some(*start),
            Frame::Code { .. } => None,
        }) {
            return Err(error(self.source, start, Unterminated::Template));
        }
        self.pending.push_back(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.pos,
                end: self.pos,
            },
        });
        Ok(())
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.pending.pop_front() {
            return Some(Ok(token));
        }
        if self.finished {
            return None;
        }
        match self.step() {
            Ok(()) => self.pending.pop_front().map(Ok),
            Err(error) => {
                self.finished = true;
                Some(Err(error))
            }
        }
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    Lexer::new(source, 0).collect()
}

fn template_pos(
    source: &str,
    pos: usize,
    start: usize,
    tokens: &mut VecDeque<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, ParseError> {
    match template_step(source, pos, start)? {
        TemplateStep::Interpolation {
            chunk_end,
            marker_end,
        } => {
            push_chunk(source, pos, chunk_end, tokens);
            tokens.push_back(Token {
                kind: TokenKind::TemplateInterpStart,
                span: Span {
                    start: chunk_end,
                    end: marker_end,
                },
            });
            frames.push(Frame::Code {
                braces: 0,
                interpolation: true,
            });
            Ok(marker_end)
        }
        TemplateStep::End(end) => {
            push_chunk(source, pos, end, tokens);
            tokens.push_back(Token {
                kind: TokenKind::TemplateEnd,
                span: Span {
                    start: end,
                    end: end + 1,
                },
            });
            frames.pop();
            Ok(end + 1)
        }
    }
}

fn push_chunk(source: &str, start: usize, end: usize, tokens: &mut VecDeque<Token>) {
    if end > start {
        tokens.push_back(Token {
            kind: TokenKind::TemplateChunk(source[start..end].to_owned()),
            span: Span { start, end },
        });
    }
}

fn code_pos(
    source: &str,
    pos: usize,
    tokens: &mut VecDeque<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, ParseError> {
    let ch = source[pos..]
        .chars()
        .next()
        .expect("pos is on a char boundary");
    let start = pos;
    if ch.is_whitespace() {
        return Ok(pos + ch.len_utf8());
    }
    if source[start..].starts_with("//") {
        return Ok(source[start..]
            .find('\n')
            .map_or(source.len(), |offset| start + offset));
    }
    if ch.is_ascii_alphabetic() {
        let end = ident_end(source, start);
        tokens.push_back(Token {
            kind: TokenKind::Ident(source[start..end].to_owned()),
            span: Span { start, end },
        });
        return Ok(end);
    }
    if ch.is_ascii_digit() {
        let end = number_end(source, start);
        tokens.push_back(Token {
            kind: TokenKind::Number(
                source[start..end]
                    .parse()
                    .expect("lexer accepts only numbers"),
            ),
            span: Span { start, end },
        });
        return Ok(end);
    }
    if ch == '"' {
        return string_pos(source, start, tokens);
    }
    punct_pos(source, start, ch, tokens, frames)
}

fn ident_end(source: &str, start: usize) -> usize {
    let mut pos = start;
    while let Some(next) = source[pos..].chars().next() {
        if !(next.is_ascii_alphanumeric() || next == '_') {
            break;
        }
        pos += next.len_utf8();
    }
    pos
}

fn number_end(source: &str, start: usize) -> usize {
    let pos = digits_end(source, start);
    let fraction = source[pos..]
        .strip_prefix('.')
        .and_then(|rest| rest.chars().next())
        .is_some_and(|next| next.is_ascii_digit());
    if fraction {
        digits_end(source, pos + 1)
    } else {
        pos
    }
}

fn string_pos(
    source: &str,
    start: usize,
    tokens: &mut VecDeque<Token>,
) -> Result<usize, ParseError> {
    let rest = &source[start + 1..];
    let quote = rest.find('"');
    let newline = rest.find('\n');
    match quote {
        Some(offset) if newline.is_none_or(|limit| offset < limit) => {
            let value_end = start + 1 + offset;
            tokens.push_back(Token {
                kind: TokenKind::String(source[start + 1..value_end].to_owned()),
                span: Span {
                    start,
                    end: value_end + 1,
                },
            });
            Ok(value_end + 1)
        }
        _ => Err(error(source, start, Unterminated::String)),
    }
}

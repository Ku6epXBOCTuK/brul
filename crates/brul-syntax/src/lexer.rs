use std::fmt;

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
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy)]
enum Frame {
    Code { braces: usize, interpolation: bool },
    Template { start: usize },
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut frames = vec![Frame::Code {
        braces: 0,
        interpolation: false,
    }];
    let mut pos = 0;
    while pos < source.len() {
        pos = match *frames.last().expect("code frame is always present") {
            Frame::Template { start } => {
                template_pos(source, pos, start, &mut tokens, &mut frames)?
            }
            Frame::Code { .. } => code_pos(source, pos, &mut tokens, &mut frames)?,
        };
    }
    if let Some(start) = frames.iter().find_map(|frame| match frame {
        Frame::Template { start } => Some(*start),
        Frame::Code { .. } => None,
    }) {
        return Err(error(source, start, "unterminated template"));
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: pos,
            end: pos,
        },
    });
    Ok(tokens)
}

fn template_pos(
    source: &str,
    pos: usize,
    start: usize,
    tokens: &mut Vec<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, Error> {
    match template_step(source, pos, start)? {
        TemplateStep::Interpolation {
            chunk_end,
            marker_end,
        } => {
            push_chunk(source, pos, chunk_end, tokens);
            tokens.push(Token {
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
            tokens.push(Token {
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

fn push_chunk(source: &str, start: usize, end: usize, tokens: &mut Vec<Token>) {
    if end > start {
        tokens.push(Token {
            kind: TokenKind::TemplateChunk(source[start..end].to_owned()),
            span: Span { start, end },
        });
    }
}

fn code_pos(
    source: &str,
    pos: usize,
    tokens: &mut Vec<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, Error> {
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
        tokens.push(Token {
            kind: TokenKind::Ident(source[start..end].to_owned()),
            span: Span { start, end },
        });
        return Ok(end);
    }
    if ch.is_ascii_digit() {
        let end = number_end(source, start);
        tokens.push(Token {
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

fn string_pos(source: &str, start: usize, tokens: &mut Vec<Token>) -> Result<usize, Error> {
    let rest = &source[start + 1..];
    let quote = rest.find('"');
    let newline = rest.find('\n');
    match quote {
        Some(offset) if newline.is_none_or(|limit| offset < limit) => {
            let value_end = start + 1 + offset;
            tokens.push(Token {
                kind: TokenKind::String(source[start + 1..value_end].to_owned()),
                span: Span {
                    start,
                    end: value_end + 1,
                },
            });
            Ok(value_end + 1)
        }
        _ => Err(error(source, start, "unterminated string")),
    }
}

fn punct_pos(
    source: &str,
    start: usize,
    ch: char,
    tokens: &mut Vec<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, Error> {
    let (kind, len) = match ch {
        '`' => {
            tokens.push(Token {
                kind: TokenKind::TemplateStart,
                span: Span {
                    start,
                    end: start + 1,
                },
            });
            frames.push(Frame::Template { start });
            return Ok(start + 1);
        }
        '{' => {
            if let Some(Frame::Code { braces, .. }) = frames.last_mut() {
                *braces += 1;
            }
            (TokenKind::Punct(Punct::LBrace), 1)
        }
        '}' => {
            let (kind, pop) = close_brace(frames);
            if pop {
                frames.pop();
            }
            (kind, 1)
        }
        _ => {
            let (punct, len) = punct_token(ch, source[start..].chars().nth(1))
                .ok_or_else(|| error(source, start, &format!("unknown character {ch:?}")))?;
            (TokenKind::Punct(punct), len)
        }
    };
    tokens.push(Token {
        kind,
        span: Span {
            start,
            end: start + len,
        },
    });
    Ok(start + len)
}

fn close_brace(frames: &mut [Frame]) -> (TokenKind, bool) {
    match frames.last_mut() {
        Some(Frame::Code { braces, .. }) if *braces > 0 => {
            *braces -= 1;
            (TokenKind::Punct(Punct::RBrace), false)
        }
        Some(Frame::Code {
            interpolation: true,
            ..
        }) => (TokenKind::TemplateInterpEnd, true),
        _ => (TokenKind::Punct(Punct::RBrace), false),
    }
}

enum TemplateStep {
    Interpolation { chunk_end: usize, marker_end: usize },
    End(usize),
}

fn template_step(source: &str, pos: usize, start: usize) -> Result<TemplateStep, Error> {
    let rest = &source[pos..];
    let tick = rest.find('`');
    let interpolation = rest.find("${");
    match (tick, interpolation) {
        (Some(tick), Some(interp)) if interp < tick => interpolation_step(source, pos, interp),
        (Some(tick), _) => end_step(source, pos, tick),
        (None, Some(interp)) => interpolation_step(source, pos, interp),
        (None, None) => {
            check_chunk(source, pos, rest)?;
            Err(error(source, start, "unterminated template"))
        }
    }
}

fn interpolation_step(source: &str, pos: usize, offset: usize) -> Result<TemplateStep, Error> {
    check_chunk(source, pos, &source[pos..pos + offset])?;
    Ok(TemplateStep::Interpolation {
        chunk_end: pos + offset,
        marker_end: pos + offset + 2,
    })
}

fn end_step(source: &str, pos: usize, offset: usize) -> Result<TemplateStep, Error> {
    check_chunk(source, pos, &source[pos..pos + offset])?;
    Ok(TemplateStep::End(pos + offset))
}

fn check_chunk(source: &str, start: usize, chunk: &str) -> Result<(), Error> {
    match chunk.find('$') {
        Some(offset) => Err(error(source, start + offset, "expected '{' after '$'")),
        None => Ok(()),
    }
}

fn error(source: &str, start: usize, message: &str) -> Error {
    let (line, column) = Span {
        start,
        end: start + 1,
    }
    .line_col(source);
    Error {
        message: message.to_owned(),
        line,
        column,
    }
}

fn digits_end(source: &str, mut pos: usize) -> usize {
    while source[pos..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        pos += 1;
    }
    pos
}

const TWO_CHAR: &[(char, char, Punct)] = &[
    ('|', '|', Punct::Or),
    ('&', '&', Punct::And),
    ('=', '=', Punct::Eq),
    ('!', '=', Punct::NotEq),
    ('<', '=', Punct::Le),
    ('>', '=', Punct::Ge),
    ('+', '=', Punct::PlusAssign),
    ('-', '=', Punct::MinusAssign),
];

const SINGLE: &[(char, Punct)] = &[
    ('=', Punct::Assign),
    ('?', Punct::Question),
    (':', Punct::Colon),
    ('<', Punct::Lt),
    ('>', Punct::Gt),
    ('+', Punct::Plus),
    ('-', Punct::Minus),
    ('*', Punct::Star),
    ('/', Punct::Slash),
    ('%', Punct::Percent),
    ('!', Punct::Bang),
    ('.', Punct::Dot),
    (',', Punct::Comma),
    (';', Punct::Semicolon),
    ('(', Punct::LParen),
    (')', Punct::RParen),
    ('[', Punct::LBracket),
    (']', Punct::RBracket),
    ('{', Punct::LBrace),
    ('}', Punct::RBrace),
];

fn punct_token(ch: char, next: Option<char>) -> Option<(Punct, usize)> {
    if let Some((_, _, punct)) = next.and_then(|next| {
        TWO_CHAR
            .iter()
            .find(|(first, second, _)| *first == ch && *second == next)
    }) {
        return Some((*punct, 2));
    }
    SINGLE
        .iter()
        .find(|(single, _)| *single == ch)
        .map(|(_, punct)| (*punct, 1))
}

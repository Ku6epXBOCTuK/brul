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
            "строка {}, колонка {}: {}",
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
        match *frames.last().expect("code frame is always present") {
            Frame::Template { start } => match template_step(source, pos, start)? {
                TemplateStep::Interpolation {
                    chunk_end,
                    marker_end,
                } => {
                    if chunk_end > pos {
                        tokens.push(Token {
                            kind: TokenKind::TemplateChunk(source[pos..chunk_end].to_owned()),
                            span: Span {
                                start: pos,
                                end: chunk_end,
                            },
                        });
                    }
                    tokens.push(Token {
                        kind: TokenKind::TemplateInterpStart,
                        span: Span {
                            start: chunk_end,
                            end: marker_end,
                        },
                    });
                    pos = marker_end;
                    frames.push(Frame::Code {
                        braces: 0,
                        interpolation: true,
                    });
                }
                TemplateStep::End(end) => {
                    if end > pos {
                        tokens.push(Token {
                            kind: TokenKind::TemplateChunk(source[pos..end].to_owned()),
                            span: Span { start: pos, end },
                        });
                    }
                    tokens.push(Token {
                        kind: TokenKind::TemplateEnd,
                        span: Span {
                            start: end,
                            end: end + 1,
                        },
                    });
                    pos = end + 1;
                    frames.pop();
                }
            },
            Frame::Code { .. } => {
                let start = pos;
                let ch = source[pos..].chars().next().unwrap();
                if ch.is_whitespace() {
                    pos += ch.len_utf8();
                    continue;
                }
                if source[start..].starts_with("//") {
                    pos = source[start..]
                        .find('\n')
                        .map_or(source.len(), |offset| start + offset);
                    continue;
                }
                if ch.is_ascii_alphabetic() {
                    pos += ch.len_utf8();
                    while let Some(next) = source[pos..].chars().next() {
                        if !(next.is_ascii_alphanumeric() || next == '_') {
                            break;
                        }
                        pos += next.len_utf8();
                    }
                    tokens.push(Token {
                        kind: TokenKind::Ident(source[start..pos].to_owned()),
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                if ch.is_ascii_digit() {
                    pos = digits_end(source, start);
                    if source[pos..].starts_with('.')
                        && source[pos + 1..]
                            .chars()
                            .next()
                            .is_some_and(|next| next.is_ascii_digit())
                    {
                        pos = digits_end(source, pos + 1);
                    }
                    tokens.push(Token {
                        kind: TokenKind::Number(
                            source[start..pos]
                                .parse()
                                .expect("lexer accepts only numbers"),
                        ),
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                if ch == '"' {
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
                            pos = value_end + 1;
                        }
                        _ => return Err(error(source, start, "незакрытая строка")),
                    }
                    continue;
                }
                if ch == '`' {
                    tokens.push(Token {
                        kind: TokenKind::TemplateStart,
                        span: Span {
                            start,
                            end: start + 1,
                        },
                    });
                    pos += 1;
                    frames.push(Frame::Template { start });
                    continue;
                }
                if ch == '{' {
                    if let Some(Frame::Code { braces, .. }) = frames.last_mut() {
                        *braces += 1;
                    }
                    pos += 1;
                    tokens.push(Token {
                        kind: TokenKind::Punct(Punct::LBrace),
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                if ch == '}' {
                    let (kind, pop) = match frames.last_mut() {
                        Some(Frame::Code {
                            braces,
                            interpolation,
                        }) if *braces > 0 => {
                            *braces -= 1;
                            (TokenKind::Punct(Punct::RBrace), false)
                        }
                        Some(Frame::Code {
                            interpolation: true,
                            ..
                        }) => (TokenKind::TemplateInterpEnd, true),
                        _ => (TokenKind::Punct(Punct::RBrace), false),
                    };
                    if pop {
                        frames.pop();
                    }
                    pos += 1;
                    tokens.push(Token {
                        kind,
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                if let Some((punct, len)) = two_char_punct(ch, source[start..].chars().nth(1)) {
                    pos += len;
                    tokens.push(Token {
                        kind: TokenKind::Punct(punct),
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                if let Some(punct) = single_punct(ch) {
                    pos += ch.len_utf8();
                    tokens.push(Token {
                        kind: TokenKind::Punct(punct),
                        span: Span { start, end: pos },
                    });
                    continue;
                }
                return Err(error(source, start, &format!("неизвестный символ {ch:?}")));
            }
        }
    }
    if let Some(start) = frames.iter().find_map(|frame| match frame {
        Frame::Template { start } => Some(*start),
        Frame::Code { .. } => None,
    }) {
        return Err(error(source, start, "незакрытый шаблон"));
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

enum TemplateStep {
    Interpolation { chunk_end: usize, marker_end: usize },
    End(usize),
}

fn template_step(source: &str, pos: usize, start: usize) -> Result<TemplateStep, Error> {
    let rest = &source[pos..];
    let tick = rest.find('`');
    let interpolation = rest.find("${");
    let boundary = match (tick, interpolation) {
        (Some(tick), Some(interp)) if interp < tick => Some((interp, true)),
        (Some(tick), _) => Some((tick, false)),
        (None, Some(interp)) => Some((interp, true)),
        (None, None) => None,
    };
    match boundary {
        Some((offset, true)) => {
            check_chunk(source, pos, &rest[..offset])?;
            Ok(TemplateStep::Interpolation {
                chunk_end: pos + offset,
                marker_end: pos + offset + 2,
            })
        }
        Some((offset, false)) => {
            check_chunk(source, pos, &rest[..offset])?;
            Ok(TemplateStep::End(pos + offset))
        }
        None => {
            check_chunk(source, pos, rest)?;
            Err(error(source, start, "незакрытый шаблон"))
        }
    }
}

fn check_chunk(source: &str, start: usize, chunk: &str) -> Result<(), Error> {
    match chunk.find('$') {
        Some(offset) => Err(error(source, start + offset, "ожидалось '{' после '$'")),
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

fn two_char_punct(ch: char, next: Option<char>) -> Option<(Punct, usize)> {
    let punct = match (ch, next?) {
        ('|', '|') => Punct::Or,
        ('&', '&') => Punct::And,
        ('=', '=') => Punct::Eq,
        ('!', '=') => Punct::NotEq,
        ('<', '=') => Punct::Le,
        ('>', '=') => Punct::Ge,
        ('+', '=') => Punct::PlusAssign,
        ('-', '=') => Punct::MinusAssign,
        _ => return None,
    };
    Some((punct, 2))
}

fn single_punct(ch: char) -> Option<Punct> {
    Some(match ch {
        '=' => Punct::Assign,
        '?' => Punct::Question,
        ':' => Punct::Colon,
        '<' => Punct::Lt,
        '>' => Punct::Gt,
        '+' => Punct::Plus,
        '-' => Punct::Minus,
        '*' => Punct::Star,
        '/' => Punct::Slash,
        '%' => Punct::Percent,
        '!' => Punct::Bang,
        '.' => Punct::Dot,
        ',' => Punct::Comma,
        ';' => Punct::Semicolon,
        '(' => Punct::LParen,
        ')' => Punct::RParen,
        '[' => Punct::LBracket,
        ']' => Punct::RBracket,
        '{' => Punct::LBrace,
        '}' => Punct::RBrace,
        _ => return None,
    })
}

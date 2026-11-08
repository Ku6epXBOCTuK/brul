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

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Number(f64),
    Unknown(char),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    while let Some(ch) = source[pos..].chars().next() {
        let start = pos;
        if ch.is_whitespace() {
            pos += ch.len_utf8();
        } else if source[start..].starts_with("//") {
            pos = source[start..]
                .find('\n')
                .map_or(source.len(), |offset| start + offset);
        } else if ch.is_ascii_alphabetic() {
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
        } else if ch.is_ascii_digit() {
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
        } else {
            pos += ch.len_utf8();
            tokens.push(Token {
                kind: TokenKind::Unknown(ch),
                span: Span { start, end: pos },
            });
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span {
            start: pos,
            end: pos,
        },
    });
    tokens
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

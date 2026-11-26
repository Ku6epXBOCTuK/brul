use super::*;
pub(super) fn punct_pos(
    source: &str,
    start: usize,
    ch: char,
    tokens: &mut VecDeque<Token>,
    frames: &mut Vec<Frame>,
) -> Result<usize, ParseError> {
    let (kind, len) = match ch {
        '`' => {
            tokens.push_back(Token {
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
                .ok_or_else(|| error(source, start, ParseError::UnknownCharacter(ch)))?;
            (TokenKind::Punct(punct), len)
        }
    };
    tokens.push_back(Token {
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

pub(super) enum TemplateStep {
    Interpolation { chunk_end: usize, marker_end: usize },
    End(usize),
}

pub(super) fn template_step(
    source: &str,
    pos: usize,
    start: usize,
) -> Result<TemplateStep, ParseError> {
    let rest = &source[pos..];
    let tick = rest.find('`');
    let interpolation = rest.find("${");
    match (tick, interpolation) {
        (Some(tick), Some(interp)) if interp < tick => interpolation_step(source, pos, interp),
        (Some(tick), _) => end_step(source, pos, tick),
        (None, Some(interp)) => interpolation_step(source, pos, interp),
        (None, None) => {
            check_chunk(source, pos, rest)?;
            Err(error(source, start, Unterminated::Template))
        }
    }
}

fn interpolation_step(source: &str, pos: usize, offset: usize) -> Result<TemplateStep, ParseError> {
    check_chunk(source, pos, &source[pos..pos + offset])?;
    Ok(TemplateStep::Interpolation {
        chunk_end: pos + offset,
        marker_end: pos + offset + 2,
    })
}

fn end_step(source: &str, pos: usize, offset: usize) -> Result<TemplateStep, ParseError> {
    check_chunk(source, pos, &source[pos..pos + offset])?;
    Ok(TemplateStep::End(pos + offset))
}

fn check_chunk(source: &str, start: usize, chunk: &str) -> Result<(), ParseError> {
    match chunk.find('$') {
        Some(offset) => Err(error(source, start + offset, ParseError::DollarBrace)),
        None => Ok(()),
    }
}

pub(super) fn error(source: &str, start: usize, reason: impl Into<ParseError>) -> ParseError {
    ParseError::at_span(
        source,
        Span {
            start,
            end: start + 1,
        },
        reason,
    )
}

pub(super) fn digits_end(source: &str, mut pos: usize) -> usize {
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
    ('@', Punct::At),
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

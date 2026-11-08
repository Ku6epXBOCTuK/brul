use brul_syntax::lexer::{Span, TokenKind, tokenize};

fn kinds(source: &str) -> Vec<TokenKind> {
    tokenize(source)
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn identifiers_and_numbers() {
    assert_eq!(
        kinds("camelCase item_2 42 2.5"),
        vec![
            TokenKind::Ident("camelCase".to_owned()),
            TokenKind::Ident("item_2".to_owned()),
            TokenKind::Number(42.0),
            TokenKind::Number(2.5),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn whitespace_and_comments_are_skipped() {
    assert_eq!(
        kinds("// note\n\t first // tail\n second"),
        vec![
            TokenKind::Ident("first".to_owned()),
            TokenKind::Ident("second".to_owned()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(kinds("// только комментарий"), vec![TokenKind::Eof]);
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
}

#[test]
fn spans_cover_tokens() {
    let source = "alpha 7\n// note\nbeta";
    let tokens = tokenize(source);
    assert_eq!(tokens[0].span, Span { start: 0, end: 5 });
    assert_eq!(tokens[1].span, Span { start: 6, end: 7 });
    assert_eq!(tokens[2].span, Span { start: 16, end: 20 });
    assert_eq!(tokens[3].span, Span { start: 20, end: 20 });
}

#[test]
fn line_col_points_at_token_start() {
    let source = "alpha 7\n// note\nbeta";
    let tokens = tokenize(source);
    assert_eq!(tokens[1].span.line_col(source), (1, 7));
    assert_eq!(tokens[2].span.line_col(source), (3, 1));
}

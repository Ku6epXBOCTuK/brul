use brul_syntax::lexer::{Punct, Span, TokenKind, tokenize};

fn kinds(source: &str) -> Vec<TokenKind> {
    tokenize(source)
        .unwrap()
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

fn punct(source: &str) -> Vec<Punct> {
    kinds(source)
        .into_iter()
        .filter_map(|kind| match kind {
            TokenKind::Punct(punct) => Some(punct),
            _ => None,
        })
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
    assert_eq!(kinds("// note"), vec![TokenKind::Eof]);
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
}

#[test]
fn punctuators_cover_grammar() {
    assert_eq!(
        punct("= += -= ? : || && == != < > <= >= + - * / % ! . , ; ( ) [ ] { }"),
        vec![
            Punct::Assign,
            Punct::PlusAssign,
            Punct::MinusAssign,
            Punct::Question,
            Punct::Colon,
            Punct::Or,
            Punct::And,
            Punct::Eq,
            Punct::NotEq,
            Punct::Lt,
            Punct::Gt,
            Punct::Le,
            Punct::Ge,
            Punct::Plus,
            Punct::Minus,
            Punct::Star,
            Punct::Slash,
            Punct::Percent,
            Punct::Bang,
            Punct::Dot,
            Punct::Comma,
            Punct::Semicolon,
            Punct::LParen,
            Punct::RParen,
            Punct::LBracket,
            Punct::RBracket,
            Punct::LBrace,
            Punct::RBrace,
        ]
    );
}

#[test]
fn strings_and_templates_keep_their_text() {
    assert_eq!(
        kinds("\"hello\" `a${name}c` \"\""),
        vec![
            TokenKind::String("hello".to_owned()),
            TokenKind::TemplateStart,
            TokenKind::TemplateChunk("a".to_owned()),
            TokenKind::TemplateInterpStart,
            TokenKind::Ident("name".to_owned()),
            TokenKind::TemplateInterpEnd,
            TokenKind::TemplateChunk("c".to_owned()),
            TokenKind::TemplateEnd,
            TokenKind::String(String::new()),
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("`line\nnext`"),
        vec![
            TokenKind::TemplateStart,
            TokenKind::TemplateChunk("line\nnext".to_owned()),
            TokenKind::TemplateEnd,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        kinds("``"),
        vec![
            TokenKind::TemplateStart,
            TokenKind::TemplateEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn nested_templates_tokenize_by_depth() {
    assert_eq!(
        kinds("`a${`b${c}`}`"),
        vec![
            TokenKind::TemplateStart,
            TokenKind::TemplateChunk("a".to_owned()),
            TokenKind::TemplateInterpStart,
            TokenKind::TemplateStart,
            TokenKind::TemplateChunk("b".to_owned()),
            TokenKind::TemplateInterpStart,
            TokenKind::Ident("c".to_owned()),
            TokenKind::TemplateInterpEnd,
            TokenKind::TemplateEnd,
            TokenKind::TemplateInterpEnd,
            TokenKind::TemplateEnd,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn slash_is_punctuation_unless_doubled() {
    assert_eq!(
        kinds("a / b // c"),
        vec![
            TokenKind::Ident("a".to_owned()),
            TokenKind::Punct(Punct::Slash),
            TokenKind::Ident("b".to_owned()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn spans_cover_tokens() {
    let source = "alpha 7\n// note\nbeta";
    let tokens = tokenize(source).unwrap();
    assert_eq!(tokens[0].span, Span { start: 0, end: 5 });
    assert_eq!(tokens[1].span, Span { start: 6, end: 7 });
    assert_eq!(tokens[2].span, Span { start: 16, end: 20 });
    assert_eq!(tokens[3].span, Span { start: 20, end: 20 });
}

#[test]
fn literal_spans_include_delimiters() {
    let source = "\"hi\" `x`";
    let tokens = tokenize(source).unwrap();
    assert_eq!(tokens[0].span, Span { start: 0, end: 4 });
    assert_eq!(tokens[1].span, Span { start: 5, end: 6 });
    assert_eq!(tokens[2].span, Span { start: 6, end: 7 });
    assert_eq!(tokens[3].span, Span { start: 7, end: 8 });
    assert_eq!(tokens[4].span, Span { start: 8, end: 8 });
}

#[test]
fn line_col_points_at_token_start() {
    let source = "alpha 7\n// note\nbeta";
    let tokens = tokenize(source).unwrap();
    assert_eq!(tokens[1].span.line_col(source), (1, 7));
    assert_eq!(tokens[2].span.line_col(source), (3, 1));
}

#[test]
fn unterminated_literals_are_errors() {
    let error = tokenize("\"abc").unwrap_err();
    assert_eq!((error.line, error.column), (1, 1));
    assert_eq!(error.message, "unterminated string");

    let error = tokenize("let\n  \"abc\ndef\"").unwrap_err();
    assert_eq!((error.line, error.column), (2, 3));
    assert_eq!(error.message, "unterminated string");

    let error = tokenize("`abc").unwrap_err();
    assert_eq!((error.line, error.column), (1, 1));
    assert_eq!(error.message, "unterminated template");

    let error = tokenize("`a${b").unwrap_err();
    assert_eq!((error.line, error.column), (1, 1));
    assert_eq!(error.message, "unterminated template");

    let error = tokenize("`a$b`").unwrap_err();
    assert_eq!((error.line, error.column), (1, 3));
    assert_eq!(error.message, "expected '{' after '$'");
}

#[test]
fn unknown_characters_are_errors() {
    let error = tokenize("a $ b").unwrap_err();
    assert_eq!((error.line, error.column), (1, 3));
    assert_eq!(error.message, "unknown character '$'");
}

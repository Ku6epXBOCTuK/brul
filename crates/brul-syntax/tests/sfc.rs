use brul_syntax::ast::Position;
use brul_syntax::sfc::{Error, parse};

const PICKER: &str = include_str!("fixtures/picker.brul");

#[test]
fn splits_four_blocks() {
    let sfc = parse(PICKER).unwrap();
    assert!(
        sfc.uses
            .unwrap()
            .content
            .contains("import BrowserItem from")
    );
    assert!(sfc.markup.content.contains("component CheckRow"));
    assert!(sfc.markup.content.contains("window Picker"));
    assert!(sfc.style.unwrap().content.contains(".checkbox:checked"));
    assert!(sfc.script.unwrap().content.contains("function handleKey"));
}

#[test]
fn content_offsets_point_into_source() {
    let sfc = parse(PICKER).unwrap();
    for block in [sfc.uses, Some(sfc.markup), sfc.style, sfc.script]
        .into_iter()
        .flatten()
    {
        let at_offset = &PICKER[block.offset..][..block.content.len()];
        assert_eq!(at_offset, block.content);
    }
}

#[test]
fn style_and_script_are_optional() {
    let sfc = parse("<markup>component A { }</markup>").unwrap();
    assert_eq!(sfc.markup.content, "component A { }");
    assert!(sfc.uses.is_none());
    assert!(sfc.style.is_none());
    assert!(sfc.script.is_none());
}

#[test]
fn empty_markup_is_allowed() {
    let sfc = parse("<markup></markup>").unwrap();
    assert_eq!(sfc.markup.content, "");
}

#[test]
fn missing_markup_is_reported() {
    let error = parse("<script>let a = 1;</script>").unwrap_err();
    assert_eq!(error.message(), "missing <markup> block");
    assert_eq!((error.line(), error.column()), (1, 1));
}

#[test]
fn duplicate_block_is_reported() {
    let source = "<markup></markup>\n<style></style>\n<style></style>";
    let error = parse(source).unwrap_err();
    match error {
        Error::At(position, reason) => {
            assert_eq!(*reason, Error::DuplicateBlock("style"));
            assert_eq!(position.line, 3);
        }
        other => panic!("expected Duplicate, got {other:?}"),
    }
}

#[test]
fn duplicate_use_block_is_reported() {
    let source = "<use></use>\n<markup></markup>\n<use></use>";
    let error = parse(source).unwrap_err();
    match error {
        Error::At(position, reason) => {
            assert_eq!(*reason, Error::DuplicateBlock("use"));
            assert_eq!(position.line, 3);
        }
        other => panic!("expected Duplicate, got {other:?}"),
    }
}

#[test]
fn unclosed_block_is_syntax_error() {
    let error = parse("<markup>\ncomponent A { }\n").unwrap_err();
    match error {
        Error::At(position, _) => assert_eq!(position.line, 3),
        other => panic!("expected Syntax, got {other:?}"),
    }
}

#[test]
fn text_outside_blocks_is_syntax_error() {
    let error = parse("<markup></markup>\nstray\n").unwrap_err();
    assert!(
        matches!(&error, Error::At(position, _) if position.line == 2),
        "{error:?}"
    );
}

#[test]
fn errors_render_distinct_messages() {
    let errors = [
        Error::at_position(Position { line: 2, column: 3 }, Error::custom("unexpected")),
        Error::at_position(
            Position { line: 2, column: 3 },
            Error::DuplicateBlock("style"),
        ),
        Error::MissingMarkup,
    ];

    let mut messages: Vec<String> = errors.iter().map(|error| error.to_string()).collect();

    assert!(messages.iter().all(|message| !message.is_empty()));
    messages.sort();
    messages.dedup();
    assert_eq!(messages.len(), errors.len());
}

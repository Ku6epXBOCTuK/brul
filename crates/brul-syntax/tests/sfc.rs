use brul_syntax::sfc::{Error, parse};

const PICKER: &str = include_str!("fixtures/picker.brul");

#[test]
fn splits_three_blocks() {
    let sfc = parse(PICKER).unwrap();
    assert!(sfc.markup.content.contains("component CheckRow"));
    assert!(sfc.markup.content.contains("window Picker"));
    assert!(sfc.style.unwrap().content.contains(".checkbox:checked"));
    assert!(sfc.script.unwrap().content.contains("function handleKey"));
}

#[test]
fn content_offsets_point_into_source() {
    let sfc = parse(PICKER).unwrap();
    for block in [Some(sfc.markup), sfc.style, sfc.script]
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
    assert_eq!(error, Error::MissingMarkup);
}

#[test]
fn duplicate_block_is_reported() {
    let source = "<markup></markup>\n<style></style>\n<style></style>";
    let error = parse(source).unwrap_err();
    match error {
        Error::Duplicate { name, line, .. } => {
            assert_eq!(name, "style");
            assert_eq!(line, 3);
        }
        other => panic!("expected Duplicate, got {other:?}"),
    }
}

#[test]
fn unclosed_block_is_syntax_error() {
    let error = parse("<markup>\ncomponent A { }\n").unwrap_err();
    match error {
        Error::Syntax { line, .. } => assert_eq!(line, 3),
        other => panic!("expected Syntax, got {other:?}"),
    }
}

#[test]
fn text_outside_blocks_is_syntax_error() {
    let error = parse("<markup></markup>\nstray\n").unwrap_err();
    assert!(matches!(error, Error::Syntax { line: 2, .. }), "{error:?}");
}

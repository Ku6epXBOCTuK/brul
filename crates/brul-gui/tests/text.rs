use brul_gui::{Atlas, AtlasSlot, TextEngine, TextError, TextStyle};

fn style() -> TextStyle {
    TextStyle::new(16.0)
}

fn slot_has_ink(atlas: &Atlas, slot: AtlasSlot) -> bool {
    let width = atlas.width();
    for row in 0..slot.height {
        for column in 0..slot.width {
            let index = (((slot.y + row) * width + slot.x + column) * 4 + 3) as usize;
            if atlas.pixels()[index] > 0 {
                return true;
            }
        }
    }
    false
}

#[test]
fn cyrillic_text_is_shaped_and_rasterized() {
    let mut engine = TextEngine::new();
    let layout = engine
        .layout("Привет", &style(), None, 1.0)
        .expect("layout cyrillic");

    assert_eq!(layout.glyphs.len(), 6, "expected one glyph per letter");
    assert!(layout.size.width > 0, "expected non-zero width");
    assert!(layout.size.height > 0, "expected non-zero height");
    for glyph in &layout.glyphs {
        assert!(glyph.slot.width > 0, "expected non-empty glyph slot");
        assert!(
            slot_has_ink(engine.atlas(), glyph.slot),
            "expected glyph ink"
        );
    }
}

#[test]
fn emoji_is_rasterized() {
    let mut engine = TextEngine::new();
    let layout = engine
        .layout("🙂", &style(), None, 1.0)
        .expect("layout emoji");

    assert_eq!(layout.glyphs.len(), 1, "expected a single emoji glyph");
    let glyph = layout.glyphs[0];
    assert!(glyph.slot.width > 0, "expected non-empty emoji slot");
    assert!(glyph.slot.height > 0, "expected non-empty emoji slot");
    assert!(
        slot_has_ink(engine.atlas(), glyph.slot),
        "expected emoji ink"
    );
}

#[test]
fn dpi_scale_changes_physical_size_and_positions() {
    let mut engine = TextEngine::new();
    let low = engine
        .layout("Привет", &style(), None, 1.0)
        .expect("layout at scale 1");
    let high = engine
        .layout("Привет", &style(), None, 2.0)
        .expect("layout at scale 2");

    assert!(
        high.size.height >= low.size.height * 2 - 1,
        "expected doubled height"
    );
    assert!(
        high.size.width >= low.size.width * 2 - 2,
        "expected doubled width"
    );

    let low_x = low.glyphs.last().expect("low glyph").x;
    let high_x = high.glyphs.last().expect("high glyph").x;
    assert!(
        (high_x - low_x * 2).abs() <= 3,
        "expected doubled glyph position"
    );
}

#[test]
fn narrow_width_wraps_text() {
    let mut engine = TextEngine::new();
    let style = style();
    let narrow = engine
        .layout("one two three", &style, Some(40.0), 1.0)
        .expect("layout narrow");
    let wide = engine
        .layout("one two three", &style, None, 1.0)
        .expect("layout wide");

    assert!(
        narrow.size.height > wide.size.height,
        "expected wrapped text to be taller"
    );
    assert!(
        narrow.size.width <= 41,
        "expected wrapped line to fit the width"
    );
}

#[test]
fn repeated_layout_reuses_atlas_entries() {
    let mut engine = TextEngine::new();
    let first = engine
        .layout("hello", &style(), None, 1.0)
        .expect("first layout");
    let count = engine.atlas().len();
    let second = engine
        .layout("hello", &style(), None, 1.0)
        .expect("second layout");

    assert_eq!(engine.atlas().len(), count, "expected atlas reuse");
    assert_eq!(first.glyphs, second.glyphs, "expected identical placement");
}

#[test]
fn empty_text_has_line_height() {
    let mut engine = TextEngine::new();
    let style = style();
    let layout = engine.layout("", &style, None, 1.0).expect("layout empty");

    assert!(layout.glyphs.is_empty(), "expected no glyphs");
    assert_eq!(layout.size.width, 0, "expected zero width");
    assert_eq!(
        layout.size.height,
        style.line_height.ceil() as u32,
        "expected one line height"
    );
}

#[test]
fn invalid_scale_is_rejected() {
    let mut engine = TextEngine::new();
    let style = style();

    assert_eq!(
        engine.layout("x", &style, None, 0.0),
        Err(TextError::InvalidScale)
    );
    assert_eq!(
        engine.layout("x", &style, None, f32::NAN),
        Err(TextError::InvalidScale)
    );
}

#[brul::attach(
    name = "custom_editor",
    events(Key, Input, Submit, Focus, Blur, Click),
    raw_events(Ime),
    update = editor_update,
    event = editor_event,
    cleanup = editor_cleanup
)]
pub fn custom_editor(
    ctx: &brul::CapabilityContext,
    value: String,
    multiline: bool,
) -> Result<EditorState, brul::ViewError> {
    let _ = (ctx, value, multiline);
    Ok(EditorState)
}

pub struct EditorState;

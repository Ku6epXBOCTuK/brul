#[brul::attach(
    name = "editor",
    events(Key, Ime, Input, Submit, Focus, Blur, Click),
    update = editor_update,
    event = editor_event,
    cleanup = editor_cleanup
)]
pub fn editor(
    ctx: &brul::Build,
    value: String,
    multiline: bool,
) -> Result<EditorState, brul::ViewError> {
    let _ = (ctx, value, multiline);
    Ok(EditorState)
}

pub struct EditorState;

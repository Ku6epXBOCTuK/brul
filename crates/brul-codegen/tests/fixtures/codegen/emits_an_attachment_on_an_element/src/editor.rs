#[brul::attach(name = "editor", events(Key, Ime))]
pub fn editor(
    ctx: &brul::AttachCtx<'_>,
    value: String,
    multiline: bool,
) -> Result<EditorState, brul::ViewError> {
    let _ = (ctx, value, multiline);
    Ok(EditorState)
}

pub struct EditorState;

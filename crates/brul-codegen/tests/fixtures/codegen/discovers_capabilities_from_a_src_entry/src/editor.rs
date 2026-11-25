#[brul::attach(name = "custom_editor", events(Input))]
pub fn custom_editor(
    _ctx: &brul::Build,
    value: String,
) -> Result<EditorState, brul::ViewError> {
    let _ = value;
    Ok(EditorState)
}

pub struct EditorState;

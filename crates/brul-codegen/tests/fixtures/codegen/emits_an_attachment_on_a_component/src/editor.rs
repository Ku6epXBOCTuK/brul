#[brul::attach(name = "custom_editor", events(Key))]
pub fn custom_editor(ctx: &brul::CapabilityContext, value: String) -> EditorState {
    let _ = (ctx, value);
    EditorState
}

pub struct EditorState;

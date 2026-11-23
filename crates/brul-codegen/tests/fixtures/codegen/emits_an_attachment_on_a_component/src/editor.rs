#[brul::attach(name = "editor", events(Key))]
pub fn editor(ctx: &brul::AttachCtx<'_>, value: String) -> EditorState {
    let _ = (ctx, value);
    EditorState
}

pub struct EditorState;

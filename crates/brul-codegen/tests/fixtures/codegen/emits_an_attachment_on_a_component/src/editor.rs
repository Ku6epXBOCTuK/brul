#[brul::attach(name = "editor", events(Key))]
pub fn editor(ctx: &brul::Build, value: String) -> EditorState {
    let _ = (ctx, value);
    EditorState
}

pub struct EditorState;

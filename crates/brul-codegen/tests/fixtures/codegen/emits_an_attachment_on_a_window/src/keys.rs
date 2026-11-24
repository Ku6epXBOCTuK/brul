#[brul::attach(name = "keys", events(Key))]
pub fn keys(ctx: &brul::Build) -> Result<(), brul::ViewError> {
    let _ = ctx;
    Ok(())
}

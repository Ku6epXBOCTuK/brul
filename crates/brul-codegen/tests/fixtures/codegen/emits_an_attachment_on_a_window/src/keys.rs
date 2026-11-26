#[brul::attach(name = "keys", events(Key))]
pub fn keys(ctx: &brul::CapabilityContext) -> Result<(), brul::ViewError> {
    let _ = ctx;
    Ok(())
}

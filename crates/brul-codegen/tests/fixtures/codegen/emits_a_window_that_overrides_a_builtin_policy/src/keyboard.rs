#[brul::attach(name = "keyboard")]
pub fn keyboard(ctx: &brul::CapabilityContext) -> Result<(), brul::ViewError> {
    let _ = ctx;
    Ok(())
}

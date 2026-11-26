#[brul::attach(name = "waveform")]
pub fn waveform(ctx: &brul::CapabilityContext) -> Result<Waveform, brul::ViewError> {
    let _ = ctx;
    Ok(Waveform)
}

pub struct Waveform;

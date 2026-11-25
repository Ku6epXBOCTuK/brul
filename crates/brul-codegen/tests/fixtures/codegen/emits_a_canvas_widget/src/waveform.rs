#[brul::attach(name = "waveform")]
pub fn waveform(ctx: &brul::Build) -> Result<Waveform, brul::ViewError> {
    let _ = ctx;
    Ok(Waveform)
}

pub struct Waveform;

use std::process::ExitCode;

mod generated;
mod painter;

pub use painter::{
    __brul_attach_waveform_apply, __brul_attach_waveform_cleanup, __brul_attach_waveform_event,
    __brul_attach_waveform_raw_event,
};

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("canvas: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("BRUL_EXAMPLE_BACKEND") {
        Ok(value) => Some(value.parse::<brul::Backend>()?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error.into()),
    };
    let mut windows = generated::windows();
    if let Some(backend) = backend {
        for window in &mut windows {
            window.config.backend = backend;
        }
    }
    brul::run_generated(
        windows,
        generated::script(),
        generated::names(),
        None,
        &[],
        None,
    )
}

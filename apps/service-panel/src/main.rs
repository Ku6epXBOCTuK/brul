#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use brul::logging::{error, info};

use service_panel::{Config, default_path, generated};

fn main() -> ExitCode {
    brul::logging::init_with(&[env!("CARGO_PKG_NAME")]);
    std::panic::set_hook(Box::new(|info| error!(panic = %info, "panic")));

    let path = default_path();
    let config = match Config::load_or_create(&path) {
        Ok(config) => config,
        Err(failure) => {
            error!(%failure, "config is unusable");
            eprintln!("panel: {failure}");
            return ExitCode::FAILURE;
        }
    };
    info!(
        path = %path.display(),
        services = config.services.len(),
        "starting panel"
    );

    match brul::run_generated(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &[],
        None,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            error!(%failure, "panel failed");
            eprintln!("panel: {failure}");
            ExitCode::FAILURE
        }
    }
}

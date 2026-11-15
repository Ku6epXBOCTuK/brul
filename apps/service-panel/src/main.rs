#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use brul::logging::{error, info};

use service_panel::{Config, default_path, generated, service};

fn main() -> ExitCode {
    brul::logging::init_with(&[env!("CARGO_PKG_NAME")]);
    std::panic::set_hook(Box::new(|info| error!(panic = %info, "panic")));

    info!(version = env!("CARGO_PKG_VERSION"), "panel starting");

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

    let services = match service::query_all(&config.services) {
        Ok(services) => services,
        Err(failure) => {
            error!(%failure, "services are unreadable");
            eprintln!("panel: {failure}");
            return ExitCode::FAILURE;
        }
    };
    for service in &services {
        info!(
            name = %service.name,
            display = %service.display,
            state = %service.state,
            start = %service.start_type,
            "service"
        );
    }

    match brul::run_generated(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &[],
        None,
    ) {
        Ok(()) => {
            info!("panel closed");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            error!(%failure, "panel failed");
            eprintln!("panel: {failure}");
            ExitCode::FAILURE
        }
    }
}

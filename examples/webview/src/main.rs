use std::process::ExitCode;

mod generated;

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("webview: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    brul::run_generated_with_platform(
        generated::windows(),
        generated::script(),
        generated::names(),
        None,
        &[],
        None,
        brul::PlatformRunOptions {
            factory: brul::webview::factory(),
            ready: Box::new(|_| {}),
        },
    )
}

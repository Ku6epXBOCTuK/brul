use brul_gui::window::{WindowConfig, run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(WindowConfig::default())?;
    Ok(())
}

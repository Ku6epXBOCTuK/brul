fn main() -> Result<(), Box<dyn std::error::Error>> {
    brul::build::app("src/window.brul")?;
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    if release && std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        // NOTE: Cargo applies build-script link args to test binaries too, so an
        // elevated manifest in debug makes `cargo test` demand UAC.
        embed_manifest::embed_manifest_file("app.manifest")?;
    }
    Ok(())
}

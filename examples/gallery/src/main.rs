use std::process::ExitCode;

mod generated;

fn main() -> ExitCode {
    brul::logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gallery: {error}");
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

#[cfg(test)]
mod tests {
    use super::generated;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn the_gallery_has_one_window() {
        let windows = generated::windows();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].name, "KitGallery");
    }

    #[test]
    fn the_generated_view_builds() {
        let component: brul::ComponentHandle = Rc::new(RefCell::new(brul::Component::new()));
        component.borrow_mut().set_names(generated::names());
        let mut build = brul::Build::new();
        generated::kit_gallery_view(Some(generated::ThemeId::Dark), &mut build, &component)
            .expect("generated gallery view");
    }
}

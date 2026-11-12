use brul_gui::Class;
use brul_syntax::style::Stylesheet;

pub use brul_gui::style::Resolved;

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, String> {
    brul_gui::style::resolve(stylesheet, classes, theme).map_err(|error| error.to_string())
}

pub fn class(stylesheet: &Stylesheet, name: &str, theme: Option<&str>) -> Result<Class, String> {
    brul_gui::style::class(stylesheet, name, theme).map_err(|error| error.to_string())
}

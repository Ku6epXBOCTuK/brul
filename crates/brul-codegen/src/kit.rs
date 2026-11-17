use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/kit.rs"));

pub const PREFIX: &str = "brul:";

const DIR: &str = "kit";

pub fn source(name: &str) -> Option<&'static str> {
    SOURCES
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, source)| *source)
}

pub fn names() -> Vec<&'static str> {
    SOURCES.iter().map(|(name, _)| *name).collect()
}

pub fn path(name: &str) -> PathBuf {
    Path::new(DIR).join(format!("{name}.brul"))
}

pub fn is_virtual(path: &Path) -> bool {
    path.parent().is_some_and(|parent| parent == Path::new(DIR))
}

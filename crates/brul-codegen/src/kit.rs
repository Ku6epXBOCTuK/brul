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

pub fn dir() -> &'static Path {
    Path::new(ROOT)
}

pub fn is_virtual(path: &Path) -> bool {
    path.parent().is_some_and(|parent| parent == Path::new(DIR))
}

#[cfg(test)]
mod tests {
    use super::{dir, names};

    #[test]
    fn every_embedded_file_lives_in_the_kit_directory() {
        for name in names() {
            let path = dir().join(format!("{name}.brul"));
            assert!(path.is_file(), "{}", path.display());
        }
    }
}

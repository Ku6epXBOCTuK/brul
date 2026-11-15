use std::fmt;

use brul_script::Error as ScriptError;

#[derive(Debug)]
pub enum ViewError {
    Script(ScriptError),
    Style(brul_gui::style::StyleError),
    Expected {
        target: String,
        expected: &'static str,
        found: &'static str,
    },
}

impl ViewError {
    pub fn expected(
        target: impl Into<String>,
        expected: &'static str,
        found: &'static str,
    ) -> Self {
        Self::Expected {
            target: target.into(),
            expected,
            found,
        }
    }

    pub fn is_unknown_theme(&self) -> bool {
        matches!(
            self,
            Self::Style(brul_gui::style::StyleError::Snapshot(
                brul_syntax::style::SnapshotError::UnknownTheme(_)
            ))
        )
    }
}

impl fmt::Display for ViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Script(error) => error.fmt(formatter),
            Self::Style(error) => error.fmt(formatter),
            Self::Expected {
                target,
                expected,
                found,
            } => write!(formatter, "{target} expects {expected}, got {found}"),
        }
    }
}

impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Script(error) => Some(error),
            Self::Style(error) => Some(error),
            Self::Expected { .. } => None,
        }
    }
}

impl From<ScriptError> for ViewError {
    fn from(error: ScriptError) -> Self {
        Self::Script(error)
    }
}

impl From<brul_gui::style::StyleError> for ViewError {
    fn from(error: brul_gui::style::StyleError) -> Self {
        Self::Style(error)
    }
}

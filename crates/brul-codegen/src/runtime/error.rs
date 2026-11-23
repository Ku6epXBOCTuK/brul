use brul_script::Error as ScriptError;
use brul_syntax::TypeName;

#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error(transparent)]
    Style(#[from] brul_gui::style::StyleError),
    #[error("{} expects {}, got {}", .target, .expected, .found)]
    Expected {
        target: String,
        expected: &'static str,
        found: &'static str,
    },
    #[error("{0}")]
    Message(String),
}

impl ViewError {
    pub fn expected<E: TypeName + ?Sized>(
        target: impl Into<String>,
        expected: &E,
        found: &'static str,
    ) -> Self {
        Self::Expected {
            target: target.into(),
            expected: expected.type_name(),
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

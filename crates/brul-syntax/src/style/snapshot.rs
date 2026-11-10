use std::fmt;

use crate::style::ast::{Property, State, Stylesheet, Theme, Value};
use crate::style::properties::Kind;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    properties: Vec<Property>,
}

impl Snapshot {
    pub fn get(&self, kind: Kind) -> Option<&Value> {
        self.properties
            .iter()
            .find(|property| property.kind == kind)
            .map(|property| &property.value)
    }

    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    fn merge(
        &mut self,
        properties: &[Property],
        theme: Option<&Theme>,
    ) -> Result<(), SnapshotError> {
        for property in properties {
            let value = resolve(&property.value, theme)?;
            match self
                .properties
                .iter_mut()
                .find(|existing| existing.kind == property.kind)
            {
                Some(existing) => existing.value = value,
                None => self.properties.push(Property {
                    kind: property.kind,
                    value,
                }),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    UnknownTheme(String),
    UnknownThemeKey(String),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::UnknownTheme(name) => write!(f, "unknown theme '{name}'"),
            SnapshotError::UnknownThemeKey(name) => write!(f, "unknown theme key '{name}'"),
        }
    }
}

impl std::error::Error for SnapshotError {}

impl Stylesheet {
    pub fn snapshot(
        &self,
        classes: &[&str],
        state: Option<State>,
        theme: Option<&str>,
    ) -> Result<Snapshot, SnapshotError> {
        let theme = match theme {
            Some(name) => Some(
                self.themes
                    .iter()
                    .find(|theme| theme.name == name)
                    .ok_or_else(|| SnapshotError::UnknownTheme(name.to_string()))?,
            ),
            None => self.themes.first(),
        };
        let mut snapshot = Snapshot::default();
        for class in classes {
            for rule in self.rules.iter().filter(|rule| rule.class == *class) {
                if rule.state.is_none() {
                    snapshot.merge(&rule.properties, theme)?;
                }
            }
        }
        if let Some(state) = state {
            for class in classes {
                for rule in self
                    .rules
                    .iter()
                    .filter(|rule| rule.class == *class && rule.state == Some(state))
                {
                    snapshot.merge(&rule.properties, theme)?;
                }
            }
        }
        Ok(snapshot)
    }
}

fn theme_value<'a>(theme: Option<&'a Theme>, key: &str) -> Result<&'a Value, SnapshotError> {
    theme
        .and_then(|theme| theme.properties.iter().find(|entry| entry.key == key))
        .map(|entry| &entry.value)
        .ok_or_else(|| SnapshotError::UnknownThemeKey(key.to_string()))
}

fn resolve(value: &Value, theme: Option<&Theme>) -> Result<Value, SnapshotError> {
    match value {
        Value::ThemeRef(key) => Ok(theme_value(theme, key)?.clone()),
        value => Ok(value.clone()),
    }
}

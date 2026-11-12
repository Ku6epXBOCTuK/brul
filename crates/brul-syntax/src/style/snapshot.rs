use std::fmt;

use crate::style::ast::{Animation, Property, State, Stylesheet, Theme, Value};
use crate::style::properties::Kind;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    properties: Vec<Property>,
    animations: Vec<Animation>,
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

    pub fn animations(&self) -> &[Animation] {
        &self.animations
    }

    fn merge_animations(&mut self, animations: &[Animation]) {
        for animation in animations {
            match self
                .animations
                .iter_mut()
                .find(|existing| existing.property == animation.property)
            {
                Some(existing) => existing.duration = animation.duration,
                None => self.animations.push(animation.clone()),
            }
        }
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
        let theme = self.theme(theme)?;
        let mut snapshot = Snapshot::default();
        for class in classes {
            for rule in self
                .rules
                .iter()
                .filter(|rule| rule.class == *class && rule.state.is_none())
            {
                snapshot.merge(&rule.properties, theme)?;
                snapshot.merge_animations(&rule.animations);
            }
        }
        if let Some(state) = state {
            let stated = self.stated_in(classes, state, theme)?;
            snapshot.merge(&stated.properties, theme)?;
        }
        Ok(snapshot)
    }

    pub fn stated(
        &self,
        classes: &[&str],
        state: State,
        theme: Option<&str>,
    ) -> Result<Snapshot, SnapshotError> {
        self.stated_in(classes, state, self.theme(theme)?)
    }

    fn theme(&self, name: Option<&str>) -> Result<Option<&Theme>, SnapshotError> {
        match name {
            Some(name) => self
                .themes
                .iter()
                .find(|theme| theme.name == name)
                .map(Some)
                .ok_or_else(|| SnapshotError::UnknownTheme(name.to_string())),
            None => Ok(self.themes.first()),
        }
    }

    fn stated_in(
        &self,
        classes: &[&str],
        state: State,
        theme: Option<&Theme>,
    ) -> Result<Snapshot, SnapshotError> {
        let mut snapshot = Snapshot::default();
        for class in classes {
            for rule in self
                .rules
                .iter()
                .filter(|rule| rule.class == *class && rule.state == Some(state))
            {
                snapshot.merge(&rule.properties, theme)?;
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

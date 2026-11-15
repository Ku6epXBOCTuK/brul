use crate::ast::{Animation, ColorValue, Gradient, Property, State, Stylesheet, Theme, Value};
use crate::color;
use crate::properties::Kind;

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

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotError {
    #[error("unknown theme '{0}'")]
    UnknownTheme(String),
    #[error("unknown theme key '{0}'")]
    UnknownThemeKey(String),
    #[error("theme key '{0}' is not a color")]
    NotAColor(String),
    #[error("theme key cycle: {0}")]
    ThemeCycle(String),
}

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

fn resolve(value: &Value, theme: Option<&Theme>) -> Result<Value, SnapshotError> {
    match value {
        Value::ThemeRef(key) => {
            color::resolve(&ColorValue::Theme(key.clone()), theme).map(Value::Color)
        }
        Value::ColorExpr(expr) => color::resolve_expr(expr, theme).map(Value::Color),
        Value::Gradient(gradient) => Ok(Value::Gradient(Gradient {
            angle: gradient.angle,
            from: ColorValue::Color(color::resolve(&gradient.from, theme)?),
            to: ColorValue::Color(color::resolve(&gradient.to, theme)?),
        })),
        value => Ok(value.clone()),
    }
}

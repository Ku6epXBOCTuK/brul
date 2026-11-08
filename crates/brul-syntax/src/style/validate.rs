use crate::error::Error;
use crate::style::ast::{ClassRule, Theme, Value};
use crate::style::properties::{self, StatePolicy};

pub struct PropertyAt {
    pub name: String,
    pub at: usize,
}

pub struct ThemeAt {
    pub name_at: usize,
    pub properties: Vec<PropertyAt>,
}

pub fn themes(source: &str, themes: &[Theme], positions: &[ThemeAt]) -> Result<(), Error> {
    let Some(first) = themes.first() else {
        return Ok(());
    };
    for (index, (theme, position)) in themes.iter().zip(positions).enumerate().skip(1) {
        if themes[..index].iter().any(|other| other.name == theme.name) {
            return Err(Error::at(
                source,
                position.name_at,
                format!("duplicate theme '{}'", theme.name),
            ));
        }
        for (property_at, property) in position.properties.iter().zip(&theme.properties) {
            match first
                .properties
                .iter()
                .find(|item| item.name == property_at.name)
            {
                None => {
                    return Err(Error::at(
                        source,
                        property_at.at,
                        format!(
                            "theme '{}' has unknown key '{}'",
                            theme.name, property_at.name
                        ),
                    ));
                }
                Some(base) if !same_type(&base.value, &property.value) => {
                    return Err(Error::at(
                        source,
                        property_at.at,
                        format!(
                            "theme '{}' has a different type for '{}'",
                            theme.name, property_at.name
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        for base in &first.properties {
            if !position
                .properties
                .iter()
                .any(|property| property.name == base.name)
            {
                return Err(Error::at(
                    source,
                    position.name_at,
                    format!("theme '{}' is missing key '{}'", theme.name, base.name),
                ));
            }
        }
    }
    Ok(())
}

pub fn states(
    source: &str,
    rules: &[ClassRule],
    positions: &[Vec<PropertyAt>],
) -> Result<(), Error> {
    for (rule, properties) in rules.iter().zip(positions) {
        if rule.state.is_none() {
            continue;
        }
        for property in properties {
            let spec = properties::get(&property.name).expect("parsed property is in the table");
            match spec.in_state {
                StatePolicy::Forbidden => {
                    return Err(Error::at(
                        source,
                        property.at,
                        format!("property '{}' cannot change in a state", property.name),
                    ));
                }
                StatePolicy::Visual => {}
                StatePolicy::TextWeight => {
                    let fixed = rules.iter().any(|base| {
                        base.state.is_none()
                            && base.class == rule.class
                            && base.properties.iter().any(|item| item.name == "width")
                            && base.properties.iter().any(|item| item.name == "height")
                    });
                    if !fixed {
                        return Err(Error::at(
                            source,
                            property.at,
                            "font-weight in a state needs width and height in the base class",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn same_type(left: &Value, right: &Value) -> bool {
    matches!(
        (left, right),
        (Value::Number(_), Value::Number(_))
            | (Value::Px(_), Value::Px(_))
            | (Value::Percent(_), Value::Percent(_))
            | (Value::String(_), Value::String(_))
            | (Value::Keyword(_), Value::Keyword(_))
            | (Value::Color(_), Value::Color(_))
            | (Value::ThemeRef(_), Value::ThemeRef(_))
            | (Value::Gradient(_), Value::Gradient(_))
            | (Value::Border(_), Value::Border(_))
    )
}

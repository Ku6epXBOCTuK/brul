use crate::error::Error;
use crate::style::ast::{BorderColor, ClassRule, Theme, Value};
use crate::style::properties::{self, StatePolicy, ThemeType};

pub struct PropertyAt {
    pub name: String,
    pub at: usize,
}

pub struct RuleAt {
    pub class_at: usize,
    pub properties: Vec<PropertyAt>,
}

pub struct ThemeAt {
    pub name_at: usize,
    pub properties: Vec<PropertyAt>,
}

pub fn themes(source: &str, themes: &[Theme], positions: &[ThemeAt]) -> Result<(), Error> {
    for (index, (theme, position)) in themes.iter().zip(positions).enumerate() {
        if themes[..index].iter().any(|other| other.name == theme.name) {
            return Err(Error::at(
                source,
                position.name_at,
                format!("duplicate theme '{}'", theme.name),
            ));
        }
        if let Some(property) = duplicate_key(position) {
            return Err(Error::at(
                source,
                property.at,
                format!("duplicate theme key '{}'", property.name),
            ));
        }
    }
    let Some((first, _)) = themes.first().zip(positions.first()) else {
        return Ok(());
    };
    for (theme, position) in themes.iter().zip(positions).skip(1) {
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
                Some(base) if key_type(&base.value) != key_type(&property.value) => {
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

pub fn refs(
    source: &str,
    themes: &[Theme],
    rules: &[ClassRule],
    positions: &[RuleAt],
) -> Result<(), Error> {
    let Some(declared) = themes.first() else {
        return Ok(());
    };
    for (rule, position) in rules.iter().zip(positions) {
        for (property, at) in rule.properties.iter().zip(&position.properties) {
            let spec =
                properties::get(&property.name).expect("the parser rejects unknown properties");
            let Some(expected) = spec.theme_type() else {
                continue;
            };
            match &property.value {
                Value::ThemeRef(key) => {
                    check(source, declared, key, expected, &property.name, at.at)?;
                }
                Value::Border(border) => {
                    if let BorderColor::ThemeRef(key) = &border.color {
                        check(
                            source,
                            declared,
                            key,
                            ThemeType::Color,
                            &property.name,
                            at.at,
                        )?;
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn states(source: &str, rules: &[ClassRule], positions: &[RuleAt]) -> Result<(), Error> {
    for (rule, position) in rules.iter().zip(positions) {
        let Some(state) = rule.state else {
            continue;
        };
        let base = rules
            .iter()
            .any(|other| other.state.is_none() && other.class == rule.class);
        if !base {
            return Err(Error::at(
                source,
                position.class_at,
                format!(
                    "state '.{}:{}' needs a base '.{}'",
                    rule.class,
                    state.name(),
                    rule.class
                ),
            ));
        }
        for property in &position.properties {
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

fn check(
    source: &str,
    declared: &Theme,
    key: &str,
    expected: ThemeType,
    property: &str,
    at: usize,
) -> Result<(), Error> {
    let actual = declared
        .properties
        .iter()
        .find(|item| item.name == key)
        .and_then(|item| key_type(&item.value));
    match actual {
        None => Err(Error::at(source, at, format!("unknown theme key '{key}'"))),
        Some(actual) if actual != expected => Err(Error::at(
            source,
            at,
            format!(
                "theme key '{key}' is {}, but '{property}' expects {}",
                actual.name(),
                expected.name()
            ),
        )),
        Some(_) => Ok(()),
    }
}

fn duplicate_key(position: &ThemeAt) -> Option<&PropertyAt> {
    position
        .properties
        .iter()
        .enumerate()
        .find_map(|(index, property)| {
            position.properties[..index]
                .iter()
                .any(|other| other.name == property.name)
                .then_some(property)
        })
}

fn key_type(value: &Value) -> Option<ThemeType> {
    match value {
        Value::Color(_) => Some(ThemeType::Color),
        Value::Px(_) => Some(ThemeType::Px),
        _ => None,
    }
}

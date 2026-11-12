use crate::error::Error;
use crate::style::ast::{ClassRule, GradientColor, Theme, ThemeEntry, Value};
use crate::style::properties::{Kind, StatePolicy, ThemeType};

pub struct PropertyAt {
    pub at: usize,
}

pub struct RuleAt {
    pub class_at: usize,
    pub properties: Vec<PropertyAt>,
    pub animations: Vec<PropertyAt>,
}

pub struct ThemeKeyAt {
    pub key: String,
    pub at: usize,
}

pub struct ThemeAt {
    pub name_at: usize,
    pub properties: Vec<ThemeKeyAt>,
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
        if let Some(key) = duplicate_key(position) {
            return Err(Error::at(
                source,
                key.at,
                format!("duplicate theme key '{}'", key.key),
            ));
        }
    }
    let Some((first, _)) = themes.first().zip(positions.first()) else {
        return Ok(());
    };
    for (theme, position) in themes.iter().zip(positions).skip(1) {
        for (key_at, entry) in position.properties.iter().zip(&theme.properties) {
            match find_key(first, &key_at.key) {
                None => {
                    return Err(Error::at(
                        source,
                        key_at.at,
                        format!("theme '{}' has unknown key '{}'", theme.name, key_at.key),
                    ));
                }
                Some(base) if key_type(&base.value) != key_type(&entry.value) => {
                    return Err(Error::at(
                        source,
                        key_at.at,
                        format!(
                            "theme '{}' has a different type for '{}'",
                            theme.name, key_at.key
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        for base in &first.properties {
            if find_key(theme, &base.key).is_none() {
                return Err(Error::at(
                    source,
                    position.name_at,
                    format!("theme '{}' is missing key '{}'", theme.name, base.key),
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
            match &property.value {
                Value::ThemeRef(key) => {
                    let Some(expected) = property.kind.theme_type() else {
                        continue;
                    };
                    check(source, declared, key, expected, property.kind, at.at)?;
                }
                Value::Gradient(gradient) => {
                    for stop in [&gradient.from, &gradient.to] {
                        if let GradientColor::Theme(key) = stop {
                            check(
                                source,
                                declared,
                                key,
                                ThemeType::Color,
                                property.kind,
                                at.at,
                            )?;
                        }
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
        if let Some(at) = position.animations.first() {
            return Err(Error::at(
                source,
                at.at,
                "animate cannot be used in a state",
            ));
        }
        for (property, at) in rule.properties.iter().zip(&position.properties) {
            match property.kind.in_state() {
                StatePolicy::Forbidden => {
                    return Err(Error::at(
                        source,
                        at.at,
                        format!(
                            "property '{}' cannot change in a state",
                            property.kind.name()
                        ),
                    ));
                }
                StatePolicy::Visual => {}
                StatePolicy::TextWeight => {
                    let fixed = rules.iter().any(|base| {
                        base.state.is_none()
                            && base.class == rule.class
                            && base.properties.iter().any(|item| item.kind == Kind::Width)
                            && base.properties.iter().any(|item| item.kind == Kind::Height)
                    });
                    if !fixed {
                        return Err(Error::at(
                            source,
                            at.at,
                            "font-weight in a state needs width and height in the base class",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn find_key<'a>(theme: &'a Theme, key: &str) -> Option<&'a ThemeEntry> {
    theme.properties.iter().find(|entry| entry.key == key)
}

fn check(
    source: &str,
    declared: &Theme,
    key: &str,
    expected: ThemeType,
    property: Kind,
    at: usize,
) -> Result<(), Error> {
    match find_key(declared, key).and_then(|entry| key_type(&entry.value)) {
        None => Err(Error::at(source, at, format!("unknown theme key '{key}'"))),
        Some(actual) if actual != expected => Err(Error::at(
            source,
            at,
            format!(
                "theme key '{key}' is {}, but '{}' expects {}",
                actual.name(),
                property.name(),
                expected.name()
            ),
        )),
        Some(_) => Ok(()),
    }
}

fn duplicate_key(position: &ThemeAt) -> Option<&ThemeKeyAt> {
    position
        .properties
        .iter()
        .enumerate()
        .find_map(|(index, key)| {
            position.properties[..index]
                .iter()
                .any(|other| other.key == key.key)
                .then_some(key)
        })
}

fn key_type(value: &Value) -> Option<ThemeType> {
    match value {
        Value::Color(_) => Some(ThemeType::Color),
        Value::Px(_) => Some(ThemeType::Px),
        _ => None,
    }
}

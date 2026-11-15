use crate::ast::{ClassRule, Theme, ThemeEntry, Value};
use crate::properties::{Kind, StatePolicy, ThemeType};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ValidationError {
    pub at: usize,
    pub message: String,
}

impl ValidationError {
    fn at(at: usize, message: String) -> Self {
        Self { at, message }
    }
}

#[derive(Debug)]
pub struct PropertyAt {
    pub at: usize,
}

#[derive(Debug)]
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

pub fn themes(themes: &[Theme], positions: &[ThemeAt]) -> Result<(), ValidationError> {
    for (index, (theme, position)) in themes.iter().zip(positions).enumerate() {
        if themes[..index].iter().any(|other| other.name == theme.name) {
            return Err(ValidationError::at(
                position.name_at,
                format!("duplicate theme '{}'", theme.name),
            ));
        }
        if let Some(key) = duplicate_key(position) {
            return Err(ValidationError::at(
                key.at,
                format!("duplicate theme key '{}'", key.key),
            ));
        }
        theme_refs(theme, position)?;
        theme_cycles(theme, position)?;
    }
    let Some((first, _)) = themes.first().zip(positions.first()) else {
        return Ok(());
    };
    for (theme, position) in themes.iter().zip(positions).skip(1) {
        for (key_at, entry) in position.properties.iter().zip(&theme.properties) {
            match find_key(first, &key_at.key) {
                None => {
                    return Err(ValidationError::at(
                        key_at.at,
                        format!("theme '{}' has unknown key '{}'", theme.name, key_at.key),
                    ));
                }
                Some(base) if key_type(&base.value) != key_type(&entry.value) => {
                    return Err(ValidationError::at(
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
                return Err(ValidationError::at(
                    position.name_at,
                    format!("theme '{}' is missing key '{}'", theme.name, base.key),
                ));
            }
        }
    }
    Ok(())
}

pub fn refs(
    themes: &[Theme],
    rules: &[ClassRule],
    positions: &[RuleAt],
) -> Result<(), ValidationError> {
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
                    check(declared, key, expected, property.kind, at.at)?;
                }
                Value::ColorExpr(expr) => {
                    let mut keys = Vec::new();
                    expr.keys(&mut keys);
                    for key in keys {
                        check(declared, key, ThemeType::Color, property.kind, at.at)?;
                    }
                }
                Value::Gradient(gradient) => {
                    for stop in [&gradient.from, &gradient.to] {
                        let mut keys = Vec::new();
                        stop.keys(&mut keys);
                        for key in keys {
                            check(declared, key, ThemeType::Color, property.kind, at.at)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn states(rules: &[ClassRule], positions: &[RuleAt]) -> Result<(), ValidationError> {
    states_with(&[], rules, positions)
}

pub fn overrides(
    component: &str,
    target: &[ClassRule],
    rules: &[ClassRule],
    positions: &[RuleAt],
) -> Result<(), ValidationError> {
    for (rule, position) in rules.iter().zip(positions) {
        if !target.iter().any(|other| other.class == rule.class) {
            return Err(ValidationError::at(
                position.class_at,
                format!("component '{component}' has no class '.{}'", rule.class),
            ));
        }
    }
    states_with(target, rules, positions)
}

pub fn states_with(
    base: &[ClassRule],
    rules: &[ClassRule],
    positions: &[RuleAt],
) -> Result<(), ValidationError> {
    for (rule, position) in rules.iter().zip(positions) {
        let Some(state) = rule.state else {
            continue;
        };
        let base_rule = |other: &ClassRule| other.state.is_none() && other.class == rule.class;
        if !base.iter().chain(rules).any(base_rule) {
            return Err(ValidationError::at(
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
            return Err(ValidationError::at(
                at.at,
                "animate cannot be used in a state".to_string(),
            ));
        }
        for (property, at) in rule.properties.iter().zip(&position.properties) {
            match property.kind.in_state() {
                StatePolicy::Forbidden => {
                    return Err(ValidationError::at(
                        at.at,
                        format!(
                            "property '{}' cannot change in a state",
                            property.kind.name()
                        ),
                    ));
                }
                StatePolicy::Visual => {}
                StatePolicy::TextWeight => {
                    let fixed = base.iter().chain(rules).any(|other| {
                        other.state.is_none()
                            && other.class == rule.class
                            && other.properties.iter().any(|item| item.kind == Kind::Width)
                            && other
                                .properties
                                .iter()
                                .any(|item| item.kind == Kind::Height)
                    });
                    if !fixed {
                        return Err(ValidationError::at(
                            at.at,
                            "font-weight in a state needs width and height in the base class"
                                .to_string(),
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
    declared: &Theme,
    key: &str,
    expected: ThemeType,
    property: Kind,
    at: usize,
) -> Result<(), ValidationError> {
    match find_key(declared, key).and_then(|entry| key_type(&entry.value)) {
        None => Err(ValidationError::at(
            at,
            format!("unknown theme key '{key}'"),
        )),
        Some(actual) if actual != expected => Err(ValidationError::at(
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

fn theme_refs(theme: &Theme, position: &ThemeAt) -> Result<(), ValidationError> {
    for (entry, key_at) in theme.properties.iter().zip(&position.properties) {
        for key in keys(&entry.value) {
            match find_key(theme, key) {
                None => {
                    return Err(ValidationError::at(
                        key_at.at,
                        format!("theme '{}' has unknown key '{key}'", theme.name),
                    ));
                }
                Some(found) => match key_type(&found.value) {
                    Some(ThemeType::Color) => {}
                    Some(actual) => {
                        return Err(ValidationError::at(
                            key_at.at,
                            format!(
                                "theme key '{key}' is {}, but '{}' needs a color",
                                actual.name(),
                                entry.key
                            ),
                        ));
                    }
                    None => {
                        return Err(ValidationError::at(
                            key_at.at,
                            format!("theme key '{key}' is not a color"),
                        ));
                    }
                },
            }
        }
    }
    Ok(())
}

fn theme_cycles(theme: &Theme, position: &ThemeAt) -> Result<(), ValidationError> {
    for entry in &theme.properties {
        walk_key(theme, position, &entry.key, &mut Vec::new())?;
    }
    Ok(())
}

fn walk_key(
    theme: &Theme,
    position: &ThemeAt,
    key: &str,
    path: &mut Vec<String>,
) -> Result<(), ValidationError> {
    if let Some(start) = path.iter().position(|item| item == key) {
        let mut cycle = path[start..].to_vec();
        cycle.push(key.to_owned());
        let at = position
            .properties
            .iter()
            .find(|item| item.key == key)
            .map_or(position.name_at, |item| item.at);
        return Err(ValidationError::at(
            at,
            format!("theme '{}' has a cycle: {}", theme.name, cycle.join(" -> ")),
        ));
    }
    let Some(entry) = find_key(theme, key) else {
        return Ok(());
    };
    path.push(key.to_owned());
    for next in keys(&entry.value) {
        walk_key(theme, position, next, path)?;
    }
    path.pop();
    Ok(())
}

fn keys(value: &Value) -> Vec<&str> {
    let mut keys = Vec::new();
    if let Value::ColorExpr(expr) = value {
        expr.keys(&mut keys);
    }
    keys
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
        Value::Color(_) | Value::ColorExpr(_) => Some(ThemeType::Color),
        Value::Px(_) => Some(ThemeType::Px),
        _ => None,
    }
}

use brul_style::validate::{self, PropertyAt, RuleAt, ThemeAt, ThemeKeyAt};
use brul_style::{
    Animation, ClassRule, Kind, Property, State, Stylesheet, Theme, ThemeEntry, Type,
};

use crate::cursor::Cursor;
use crate::error::Error;
use crate::style::shorthand;
use crate::style::value;

#[derive(Debug)]
pub struct Style {
    pub stylesheet: Stylesheet,
    pub overrides: Vec<Override>,
}

#[derive(Debug)]
pub struct Override {
    pub component: String,
    pub component_at: usize,
    pub rules: Vec<ClassRule>,
    pub positions: Vec<RuleAt>,
}

pub fn parse_file(source: &str) -> Result<Stylesheet, Error> {
    Ok(parse(source, Scope::Standalone)?.stylesheet)
}

pub fn parse_file_in(source: &str, palette: &[Theme]) -> Result<Stylesheet, Error> {
    Ok(parse(source, Scope::Imported(palette))?.stylesheet)
}

pub fn parse_root(source: &str) -> Result<Style, Error> {
    parse(source, Scope::Root)
}

#[derive(Clone, Copy)]
enum Scope<'a> {
    Root,
    Standalone,
    Imported(&'a [Theme]),
}

impl<'a> Scope<'a> {
    fn palette(self) -> Option<&'a [Theme]> {
        match self {
            Self::Imported(palette) => Some(palette),
            Self::Root | Self::Standalone => None,
        }
    }

    fn allows_overrides(self) -> bool {
        matches!(self, Self::Root)
    }
}

fn parse(source: &str, scope: Scope<'_>) -> Result<Style, Error> {
    let mut cursor = Cursor::new(source);
    let mut themes = Vec::new();
    let mut theme_positions = Vec::new();
    let mut rules = Vec::new();
    let mut positions = Vec::new();
    let mut overrides = Vec::new();
    loop {
        cursor.skip_trivia();
        if cursor.peek().is_none() {
            break;
        }
        if cursor.eat_keyword("theme") {
            if scope.palette().is_some() {
                return Err(cursor.error(Error::ThemeNotRoot));
            }
            let (theme, position) = cursor.theme()?;
            themes.push(theme);
            theme_positions.push(position);
        } else if cursor.eat_keyword("override") {
            if !scope.allows_overrides() {
                return Err(cursor.error(Error::OverrideNotRoot));
            }
            overrides.push(cursor.override_block()?);
        } else {
            let (rule, position) = cursor.class_rule()?;
            rules.push(rule);
            positions.push(position);
        }
    }
    let report = |error: brul_style::ValidationError| {
        Error::at(source, error.at, Error::custom(error.message))
    };
    let declared = match scope.palette() {
        Some(palette) => palette,
        None => {
            validate::themes(&themes, &theme_positions).map_err(report)?;
            &themes
        }
    };
    validate::refs(declared, &rules, &positions).map_err(report)?;
    for over in &overrides {
        validate::refs(declared, &over.rules, &over.positions).map_err(report)?;
    }
    validate::states(&rules, &positions).map_err(report)?;
    Ok(Style {
        stylesheet: Stylesheet {
            themes: declared.to_vec(),
            rules,
        },
        overrides,
    })
}

impl Cursor<'_> {
    fn class_rule(&mut self) -> Result<(ClassRule, RuleAt), Error> {
        self.expect('.')?;
        let class_at = self.value_start();
        let class = self.ident()?;
        let state = self.state()?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        let mut animations = Vec::new();
        let mut animation_positions = Vec::new();
        self.braced_items(|cursor| {
            let at = cursor.value_start();
            if cursor.eat_keyword("animate") {
                animations.push(cursor.animation()?);
                animation_positions.push(PropertyAt { at });
            } else {
                for (property, at) in cursor.property()? {
                    properties.push(property);
                    positions.push(at);
                }
            }
            Ok(())
        })?;
        Ok((
            ClassRule {
                class,
                state,
                properties,
                animations,
            },
            RuleAt {
                class_at,
                properties: positions,
                animations: animation_positions,
            },
        ))
    }

    fn override_block(&mut self) -> Result<Override, Error> {
        let component_at = self.value_start();
        let component = self.ident()?;
        let mut rules = Vec::new();
        let mut positions = Vec::new();
        self.braced_items(|cursor| {
            let (rule, position) = cursor.class_rule()?;
            rules.push(rule);
            positions.push(position);
            Ok(())
        })?;
        Ok(Override {
            component,
            component_at,
            rules,
            positions,
        })
    }

    fn animation(&mut self) -> Result<Animation, Error> {
        let at = self.value_start();
        let name = self.ident()?;
        let property = Kind::from_name(&name)
            .ok_or_else(|| Error::at(self.source(), at, Error::UnknownProperty(name.clone())))?;
        if !property.animatable() {
            return Err(Error::at(
                self.source(),
                at,
                Error::NotAnimatable(name.clone()),
            ));
        }
        self.expect(':')?;
        let duration = value::duration(&mut *self, &name)?;
        self.expect(';')?;
        Ok(Animation { property, duration })
    }

    fn state(&mut self) -> Result<Option<State>, Error> {
        if !self.eat(':') {
            return Ok(None);
        }
        let at = self.value_start();
        let name = self.ident()?;
        let state = State::ALL
            .iter()
            .find(|state| state.name() == name)
            .copied()
            .ok_or_else(|| Error::at(self.source(), at, Error::UnknownState))?;
        Ok(Some(state))
    }

    fn theme(&mut self) -> Result<(Theme, ThemeAt), Error> {
        let name_at = self.value_start();
        let name = self.ident()?;
        let mut properties = Vec::new();
        let mut positions = Vec::new();
        self.braced_items(|cursor| {
            let (property, at) = cursor.theme_property()?;
            properties.push(property);
            positions.push(at);
            Ok(())
        })?;
        Ok((
            Theme { name, properties },
            ThemeAt {
                name_at,
                properties: positions,
            },
        ))
    }

    fn property(&mut self) -> Result<Vec<(Property, PropertyAt)>, Error> {
        let at = self.value_start();
        let name = self.ident()?;
        if let Some(shorthand) = shorthand::get(&name) {
            self.expect(':')?;
            let values = (shorthand.parse)(&mut *self, &name)?;
            self.expect(';')?;
            return Ok(shorthand
                .properties(values)
                .into_iter()
                .map(|property| (property, PropertyAt { at }))
                .collect());
        }
        let kind = Kind::from_name(&name)
            .ok_or_else(|| Error::at(self.source(), at, Error::UnknownProperty(name.clone())))?;
        let spec = kind.spec();
        self.expect(':')?;
        let value = value::value(&mut *self, spec.theme_type().is_some(), spec.ty, &name)?;
        self.expect(';')?;
        Ok(vec![(Property { kind, value }, PropertyAt { at })])
    }

    fn theme_property(&mut self) -> Result<(ThemeEntry, ThemeKeyAt), Error> {
        let at = self.value_start();
        let key = self.ident()?;
        self.expect(':')?;
        let value = value::value(&mut *self, false, Type::ThemeKey, &key)?;
        self.expect(';')?;
        Ok((
            ThemeEntry {
                key: key.clone(),
                value,
            },
            ThemeKeyAt { key, at },
        ))
    }
}

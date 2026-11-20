use std::sync::Arc;

use brul_syntax::TypeName;
use brul_syntax::ast::{BinaryOp, UnaryOp};

use crate::component::Component;
use crate::{Error, Items, Value, eval};

#[derive(Debug, Clone, PartialEq)]
pub struct Dyn(Value);

impl Dyn {
    pub fn null() -> Self {
        Self(Value::Null)
    }

    pub fn num(number: f64) -> Self {
        Self(Value::Number(number))
    }

    pub fn text(text: impl Into<Arc<str>>) -> Self {
        Self(Value::String(text.into()))
    }

    pub fn flag(flag: bool) -> Self {
        Self(Value::Bool(flag))
    }

    pub fn array(items: Vec<Self>) -> Self {
        Self(Value::Array(Arc::new(
            items.into_iter().map(Self::into_value).collect(),
        )))
    }

    pub fn object(fields: Vec<(String, Self)>) -> Self {
        Self(Value::Object(Arc::new(
            fields
                .into_iter()
                .map(|(name, value)| (name, value.into_value()))
                .collect(),
        )))
    }

    pub fn image_bytes(bytes: &[u8]) -> Self {
        Self(Value::image_bytes(bytes))
    }

    pub fn field(&self, name: &str) -> Result<Self, Error> {
        Ok(Self(eval::read_field(&self.0, name)?))
    }

    pub fn index(&self, position: &Self) -> Result<Self, Error> {
        Ok(Self(eval::read_index(&self.0, &position.0)?))
    }

    pub fn boolean(&self) -> Result<bool, Error> {
        bool::try_from(self)
    }

    pub fn as_number(&self) -> Result<f64, Error> {
        f64::try_from(self)
    }

    pub fn as_text(&self) -> Result<String, Error> {
        String::try_from(self)
    }

    pub fn unit(&self) -> Result<(), Error> {
        <()>::try_from(self)
    }

    pub fn unary(op: UnaryOp, value: &Self) -> Result<Self, Error> {
        Ok(Self(eval::apply_unary(op, value.0.clone())?))
    }

    pub fn binary(op: BinaryOp, left: &Self, right: &Self) -> Result<Self, Error> {
        Ok(Self(eval::apply_binary(
            op,
            left.0.clone(),
            right.0.clone(),
        )?))
    }

    pub fn template(parts: Vec<Self>) -> Result<Self, Error> {
        Ok(Self::text(eval::template(
            parts.into_iter().map(Self::into_value).collect(),
        )?))
    }

    pub fn assign_field(&mut self, name: &str, value: &Self) -> Result<(), Error> {
        eval::assign_field(&mut self.0, name, value.0.clone())
    }

    pub fn assign_index(&mut self, position: &Self, value: &Self) -> Result<(), Error> {
        eval::assign_index(&mut self.0, &position.0, value.0.clone())
    }

    pub fn call(
        component: &mut Component,
        name: &str,
        arguments: Vec<Self>,
    ) -> Result<Self, Error> {
        Ok(Self(eval::call(
            component,
            name,
            arguments.into_iter().map(Self::into_value).collect(),
        )?))
    }

    pub fn into_value(self) -> Value {
        self.0
    }
}

impl TypeName for Dyn {
    fn type_name(&self) -> &'static str {
        self.0.type_name()
    }
}

impl From<Value> for Dyn {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

impl From<Dyn> for Value {
    fn from(value: Dyn) -> Self {
        value.0
    }
}

impl From<&Dyn> for Value {
    fn from(value: &Dyn) -> Self {
        value.0.clone()
    }
}

impl From<Vec<Dyn>> for Value {
    fn from(items: Vec<Dyn>) -> Self {
        Self::Array(Arc::new(items.into_iter().map(Dyn::into_value).collect()))
    }
}

impl From<&str> for Dyn {
    fn from(text: &str) -> Self {
        Self(Value::from(text))
    }
}

impl From<String> for Dyn {
    fn from(text: String) -> Self {
        Self(Value::from(text))
    }
}

impl From<f64> for Dyn {
    fn from(number: f64) -> Self {
        Self(Value::from(number))
    }
}

impl From<bool> for Dyn {
    fn from(flag: bool) -> Self {
        Self(Value::from(flag))
    }
}

impl From<()> for Dyn {
    fn from(_: ()) -> Self {
        Self::null()
    }
}

impl TryFrom<&Dyn> for String {
    type Error = Error;

    fn try_from(value: &Dyn) -> Result<Self, Self::Error> {
        String::try_from(&value.0)
    }
}

impl TryFrom<&Dyn> for f64 {
    type Error = Error;

    fn try_from(value: &Dyn) -> Result<Self, Self::Error> {
        f64::try_from(&value.0)
    }
}

impl TryFrom<&Dyn> for bool {
    type Error = Error;

    fn try_from(value: &Dyn) -> Result<Self, Self::Error> {
        bool::try_from(&value.0)
    }
}

impl TryFrom<&Dyn> for () {
    type Error = Error;

    fn try_from(value: &Dyn) -> Result<Self, Self::Error> {
        <()>::try_from(&value.0)
    }
}

impl TryFrom<&Dyn> for usize {
    type Error = Error;

    fn try_from(value: &Dyn) -> Result<Self, Self::Error> {
        usize::try_from(&value.0)
    }
}

impl TryFrom<Dyn> for String {
    type Error = Error;

    fn try_from(value: Dyn) -> Result<Self, Self::Error> {
        String::try_from(&value)
    }
}

impl TryFrom<Dyn> for f64 {
    type Error = Error;

    fn try_from(value: Dyn) -> Result<Self, Self::Error> {
        f64::try_from(&value)
    }
}

impl TryFrom<Dyn> for bool {
    type Error = Error;

    fn try_from(value: Dyn) -> Result<Self, Self::Error> {
        bool::try_from(&value)
    }
}

impl TryFrom<Dyn> for () {
    type Error = Error;

    fn try_from(value: Dyn) -> Result<Self, Self::Error> {
        <()>::try_from(&value)
    }
}

impl TryFrom<Dyn> for usize {
    type Error = Error;

    fn try_from(value: Dyn) -> Result<Self, Self::Error> {
        usize::try_from(&value)
    }
}

pub fn items(value: &Dyn) -> Items {
    crate::value::items(&value.0)
}

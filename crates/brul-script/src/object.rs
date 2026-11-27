use std::sync::Arc;

use crate::Value;

#[derive(Debug, Clone, PartialEq, Hash)]
struct Field {
    name: &'static str,
    value: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Hash)]
pub struct Object {
    fields: Arc<Vec<Field>>,
}

impl Object {
    pub fn builder() -> ObjectBuilder {
        ObjectBuilder::default()
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    }

    pub(crate) fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        Arc::make_mut(&mut self.fields)
            .iter_mut()
            .find(|field| field.name == name)
            .map(|field| &mut field.value)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &Value> + '_ {
        self.fields.iter().map(|field| &field.value)
    }
}

#[derive(Debug, Default)]
pub struct ObjectBuilder(Vec<Field>);

impl ObjectBuilder {
    pub fn field(mut self, name: &'static str, value: impl Into<Value>) -> Self {
        self.0.push(Field {
            name,
            value: value.into(),
        });
        self
    }
}

impl From<ObjectBuilder> for Object {
    fn from(builder: ObjectBuilder) -> Self {
        Self {
            fields: Arc::new(builder.0),
        }
    }
}

impl From<Object> for Value {
    fn from(object: Object) -> Self {
        Self::Object(object)
    }
}

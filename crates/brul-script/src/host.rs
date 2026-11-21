use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

pub trait Host {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error>;

    fn commands(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

pub trait AnyHost: Host + std::any::Any {}

impl<T: Host + 'static> AnyHost for T {}

pub trait HostCommand {
    type Arguments;
    type Output;

    const NAME: &'static str;

    fn invoke(component: &mut Component, arguments: Self::Arguments)
    -> Result<Self::Output, Error>;
}

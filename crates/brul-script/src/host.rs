use crate::error::Error;
use crate::value::Value;

pub trait Host {
    fn call(&mut self, name: &str, arguments: &[Value]) -> Result<Value, Error>;

    fn every(&mut self, interval_ms: f64, function: &str) -> Result<(), Error> {
        let _ = (interval_ms, function);
        Err(Error::Unsupported("every"))
    }
}

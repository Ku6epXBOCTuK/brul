use crate::component::Component;
use crate::error::Error;
use crate::value::Value;

pub fn call(component: &mut Component, name: &str, values: Vec<Value>) -> Result<Value, Error> {
    for (candidate, function) in SCRIPT_CALL {
        if *candidate == name {
            return function(values);
        }
    }
    if component.has_dispatch() {
        return match component.call_function(name, values.clone()) {
            Err(Error::UnknownFunction(_)) => component.call(name, &values),
            result => result,
        };
    }
    component.call(name, &values)
}

include!(concat!(env!("OUT_DIR"), "/lang_script_call.rs"));

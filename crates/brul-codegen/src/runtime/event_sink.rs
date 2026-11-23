use std::sync::Arc;

use brul_script::Value;

use super::StateId;

pub trait EventSink: Send + Sync + 'static {
    fn widget(&self, state: StateId, event: &str, arguments: Vec<Value>);
}

pub fn emit(sink: &Option<Arc<dyn EventSink>>, state: StateId, event: &str, arguments: Vec<Value>) {
    if let Some(sink) = sink {
        sink.widget(state, event, arguments);
    }
}

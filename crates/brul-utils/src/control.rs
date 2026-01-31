use winit::event_loop::EventLoopProxy;

use crate::{Error, Result};

#[derive(Debug)]
pub enum AppControlMessage {
    AppStarted,
    RequestShutdown,
}

#[derive(Debug, Default)]
pub enum GuiControlMessage {
    #[default]
    Shutdown,
}

#[derive(Default, Debug)]
pub struct EVProxy {
    proxy: Option<EventLoopProxy<GuiControlMessage>>,
}

impl EVProxy {
    pub fn new() -> Self {
        Self { proxy: None }
    }

    pub fn set_proxy(&mut self, proxy: EventLoopProxy<GuiControlMessage>) {
        self.proxy = Some(proxy);
    }

    pub fn get_proxy(&self) -> Option<EventLoopProxy<GuiControlMessage>> {
        self.proxy.clone()
    }

    pub fn send(&self, msg: GuiControlMessage) -> Result<()> {
        let proxy = self
            .proxy
            .as_ref()
            .ok_or(Error::WinitOtherError("Proxy not set"))?;
        let result = proxy.send_event(msg)?;
        Ok(result)
    }
}

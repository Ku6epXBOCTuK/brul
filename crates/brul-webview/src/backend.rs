use std::sync::Arc;

use brul_window::PlatformBackendFactory;

#[cfg(not(windows))]
pub use unsupported::UnsupportedFactory as BackendFactory;
#[cfg(windows)]
pub use wry_backend::WryFactory as BackendFactory;

pub fn factory() -> Arc<dyn PlatformBackendFactory> {
    Arc::new(BackendFactory)
}

#[cfg(windows)]
mod wry_backend {
    use std::collections::HashMap;
    use std::sync::Arc;

    use brul_gui::PlatformViewId;
    use brul_window::{
        PlatformBackendFactory, PlatformMessageHandler, PlatformViewBackend, PlatformViewBounds,
        PlatformWindow, WindowError,
    };
    use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
    use wry::dpi::{PhysicalPosition, PhysicalSize};
    use wry::{Rect, WebView, WebViewBuilder};

    pub struct WryFactory;

    impl PlatformBackendFactory for WryFactory {
        fn create(
            &self,
            window: &PlatformWindow<'_>,
            messages: PlatformMessageHandler,
        ) -> Result<Box<dyn PlatformViewBackend>, WindowError> {
            Ok(Box::new(WryBackend {
                raw: window.handle().as_raw(),
                views: HashMap::new(),
                messages,
            }))
        }
    }

    struct WryBackend {
        raw: RawWindowHandle,
        views: HashMap<PlatformViewId, WebView>,
        messages: PlatformMessageHandler,
    }

    impl HasWindowHandle for WryBackend {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            // NOTE: TreeApp drops this backend before the window it borrows
            Ok(unsafe { WindowHandle::borrow_raw(self.raw) })
        }
    }

    impl PlatformViewBackend for WryBackend {
        fn create(&mut self, id: PlatformViewId, source: &str, bounds: PlatformViewBounds, _: f64) {
            let messages = Arc::clone(&self.messages);
            let webview = WebViewBuilder::new()
                .with_url(source)
                .with_bounds(rect(bounds))
                .with_ipc_handler(move |request| messages(id, request.body().to_owned()))
                .build_as_child(self);
            match webview {
                Ok(webview) => {
                    self.views.insert(id, webview);
                }
                Err(error) => tracing::warn!("webview creation failed: {error}"),
            }
        }

        fn set_bounds(&mut self, id: PlatformViewId, bounds: PlatformViewBounds, _: f64) {
            if let Some(webview) = self.views.get(&id)
                && let Err(error) = webview.set_bounds(rect(bounds))
            {
                tracing::warn!("webview bounds failed: {error}");
            }
        }

        fn set_source(&mut self, id: PlatformViewId, source: &str) {
            if let Some(webview) = self.views.get(&id)
                && let Err(error) = webview.load_url(source)
            {
                tracing::warn!("webview navigation failed: {error}");
            }
        }

        fn set_visible(&mut self, id: PlatformViewId, visible: bool) {
            if let Some(webview) = self.views.get(&id)
                && let Err(error) = webview.set_visible(visible)
            {
                tracing::warn!("webview visibility failed: {error}");
            }
        }

        fn focus(&mut self, id: PlatformViewId) {
            if let Some(webview) = self.views.get(&id)
                && let Err(error) = webview.focus()
            {
                tracing::warn!("webview focus failed: {error}");
            }
        }

        fn blur(&mut self) {
            for webview in self.views.values() {
                if let Err(error) = webview.focus_parent() {
                    tracing::warn!("webview blur failed: {error}");
                }
            }
        }

        fn remove(&mut self, id: PlatformViewId) {
            self.views.remove(&id);
        }
    }

    fn rect(bounds: PlatformViewBounds) -> Rect {
        Rect {
            position: PhysicalPosition::new(bounds.x, bounds.y).into(),
            size: PhysicalSize::new(bounds.width, bounds.height).into(),
        }
    }
}

#[cfg(not(windows))]
mod unsupported {
    use std::sync::Arc;

    use brul_window::{
        PlatformBackendFactory, PlatformMessageHandler, PlatformViewBackend, PlatformWindow,
        WindowError,
    };
    use thiserror::Error;

    pub struct UnsupportedFactory;

    #[derive(Debug, Error)]
    #[error("WebView platform views are supported only on Windows")]
    struct UnsupportedPlatformView;

    impl PlatformBackendFactory for UnsupportedFactory {
        fn create(
            &self,
            _: &PlatformWindow<'_>,
            _: PlatformMessageHandler,
        ) -> Result<Box<dyn PlatformViewBackend>, WindowError> {
            Err(WindowError::app(UnsupportedPlatformView))
        }
    }
}

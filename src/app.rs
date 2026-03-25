//! Application runner that abstracts the event loop across backends
//!
//! Provides a unified `App::run()` that works with both TUI and GUI backends.
//! The developer provides a callback that receives events and a renderer —
//! no backend-specific code needed.

use crate::event::Event;
use crate::render::Renderer;
use anyhow::Result;

/// Application runner that owns the event loop and renderer
pub struct App;

impl App {
    /// Run the application with the TUI backend
    ///
    /// Creates a `TerminalRenderer`, enters alt screen, and runs a blocking
    /// event loop. The callback receives each event and the renderer. Return
    /// `false` from the callback to exit.
    #[cfg(feature = "tui")]
    pub fn run_tui<F>(mut callback: F) -> Result<()>
    where
        F: FnMut(&Event, &mut dyn Renderer) -> bool,
    {
        use crate::event::{EventKind, EventPoller};
        use crate::tui::TerminalRenderer;

        let mut renderer = TerminalRenderer::new()?;
        renderer.enter_alt_screen()?;

        let events = EventPoller::new()?;

        loop {
            let event = events.read()?;

            if let EventKind::Resize(_, _) = &event.kind {
                renderer.refresh_geometry()?;
            }

            if !callback(&event, &mut renderer) {
                break;
            }
        }

        Ok(())
    }

    /// Run the application with the GUI backend
    ///
    /// Creates a window with a `WgpuRenderer` and runs the winit event loop.
    /// The callback receives each event and the renderer. Return `false` from
    /// the callback to exit.
    #[cfg(feature = "gui")]
    pub fn run_gui<F>(title: &str, font_size: f32, callback: F) -> Result<()>
    where
        F: FnMut(&Event, &mut dyn Renderer) -> bool + 'static,
    {
        use crate::event::convert_winit_event;
        use crate::gui::WgpuRenderer;
        use std::sync::Arc;
        use winit::application::ApplicationHandler;
        use winit::event::WindowEvent;
        use winit::event_loop::{ActiveEventLoop, EventLoop};
        use winit::window::{Window, WindowAttributes, WindowId};

        struct AppHandler<F> {
            title: String,
            font_size: f32,
            window: Option<Arc<Window>>,
            renderer: Option<WgpuRenderer>,
            callback: F,
        }

        impl<F> ApplicationHandler for AppHandler<F>
        where
            F: FnMut(&Event, &mut dyn Renderer) -> bool,
        {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window.is_some() {
                    return;
                }

                let attrs = WindowAttributes::default().with_title(&self.title);
                let window =
                    Arc::new(event_loop.create_window(attrs).expect("create window"));
                let renderer = WgpuRenderer::new(window.clone(), self.font_size)
                    .expect("create wgpu renderer");

                self.window = Some(window);
                self.renderer = Some(renderer);
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                _window_id: WindowId,
                event: WindowEvent,
            ) {
                if matches!(&event, WindowEvent::CloseRequested) {
                    self.renderer.take();
                    self.window.take();
                    event_loop.exit();
                    return;
                }

                if let WindowEvent::Resized(size) = &event {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }
                }

                if let Some(mkui_event) = convert_winit_event(&event) {
                    if let Some(renderer) = &mut self.renderer {
                        if !(self.callback)(&mkui_event, renderer) {
                            self.renderer.take();
                            self.window.take();
                            event_loop.exit();
                        }
                    }
                }
            }

            fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }

        let event_loop = EventLoop::new()?;
        let mut handler = AppHandler {
            title: title.to_string(),
            font_size,
            window: None,
            renderer: None,
            callback,
        };
        event_loop.run_app(&mut handler)?;

        Ok(())
    }
}

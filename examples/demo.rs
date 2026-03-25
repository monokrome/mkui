use anyhow::Result;
use mkui::components::{Header, StatusBar, Text};
use mkui::context::RenderContext;
use mkui::layout::{FlexDirection, FlexLayout, Rect, Size};
use mkui::render::Renderer;
use mkui::slots::Slots;
use mkui::terminal::TerminalCapabilities;
use mkui::theme::Theme;
use mkui::Component;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("--tui");

    match mode {
        "--gui" => {
            #[cfg(feature = "gui")]
            {
                run_gui()?;
            }
            #[cfg(not(feature = "gui"))]
            {
                eprintln!("GUI support not compiled. Rebuild with: cargo run --features gui --example demo -- --gui");
                std::process::exit(1);
            }
        }
        "--tui" => run_tui()?,
        other => {
            eprintln!("Unknown mode: {other}. Use --tui or --gui");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn render_frame(
    renderer: &mut dyn Renderer,
    header: &mut Header,
    content: &mut Text,
    status: &mut StatusBar,
    ctx: &RenderContext,
) -> Result<()> {
    let (cols, rows) = renderer.dimensions();
    let bounds = Rect::fullscreen(cols, rows);

    let layout = FlexLayout::new(FlexDirection::Column);
    let rects = layout.layout(bounds, &[Size::Fixed(1), Size::Flex(1), Size::Fixed(1)]);

    renderer.begin_frame()?;
    renderer.clear()?;

    header.render(renderer, rects[0], ctx)?;
    content.render(renderer, rects[1], ctx)?;
    status.render(renderer, rects[2], ctx)?;

    renderer.end_frame()?;

    Ok(())
}

fn run_tui() -> Result<()> {
    use mkui::event::{Event, EventPoller, Key};
    use mkui::render::TerminalRenderer;

    let mut renderer = TerminalRenderer::new()?;
    renderer.enter_alt_screen()?;

    let caps = TerminalCapabilities::detect();
    let theme = Theme::new(caps);
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    let events = EventPoller::new()?;

    let mut header = Header::new();
    let mut content = Text::new("TUI mode. Press 'q' or ESC to quit.");
    let mut status = StatusBar::with_text("mkui", "TUI", &theme);

    loop {
        render_frame(&mut renderer, &mut header, &mut content, &mut status, &ctx)?;

        match events.read()? {
            Event::Key(Key::Char('q') | Key::Esc) => break,
            Event::Resize(_, _) => renderer.refresh_geometry()?,
            _ => {}
        }
    }

    Ok(())
}

#[cfg(feature = "gui")]
fn run_gui() -> Result<()> {
    use mkui::gui::WgpuRenderer;
    use std::sync::Arc;
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::keyboard::{Key, NamedKey};
    use winit::window::{Window, WindowAttributes, WindowId};

    struct App {
        window: Option<Arc<Window>>,
        renderer: Option<WgpuRenderer>,
        header: Header,
        content: Text,
        status: StatusBar,
        theme: Theme,
        slots: Slots,
    }

    impl App {
        fn new() -> Self {
            let caps = TerminalCapabilities::detect();
            let theme = Theme::new(caps);
            let status = StatusBar::with_text("mkui", "GUI", &theme);

            App {
                window: None,
                renderer: None,
                header: Header::new(),
                content: Text::new("GUI mode. Press 'q' or ESC to quit."),
                status,
                theme,
                slots: Slots::new(),
            }
        }
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }

            let attrs = WindowAttributes::default().with_title("mkui demo");
            let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

            let renderer =
                WgpuRenderer::new(window.clone(), 16.0).expect("create wgpu renderer");

            self.window = Some(window);
            self.renderer = Some(renderer);
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    self.renderer.take();
                    self.window.take();
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed =>
                {
                    let should_quit = match &event.logical_key {
                        Key::Named(NamedKey::Escape) => true,
                        Key::Character(c) if c.as_str() == "q" => true,
                        _ => false,
                    };
                    if should_quit {
                        self.renderer.take();
                        self.window.take();
                        event_loop.exit();
                    }
                }
                WindowEvent::Resized(size) => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(renderer) = &mut self.renderer {
                        let ctx = RenderContext::new(&self.theme, &self.slots);
                        let _ = render_frame(
                            renderer,
                            &mut self.header,
                            &mut self.content,
                            &mut self.status,
                            &ctx,
                        );
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    let event_loop = EventLoop::new()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}

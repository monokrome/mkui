//! mkui demo - Simple window with header and status bar

use anyhow::Result;
use mkui::{
    component::Component,
    components::{Container, Header, StatusBar},
    context::RenderContext,
    event::{Event, EventPoller, Key},
    layout::Rect,
    slots::Slots,
    Renderer, Theme,
};
use std::time::Duration;

fn main() -> Result<()> {
    // Create renderer and theme
    let mut renderer = Renderer::new()?;
    let caps = renderer.context().capabilities;
    let theme = Theme::new(caps);
    renderer.enter_alt_screen()?;
    renderer.hide_cursor()?;
    renderer.clear()?;

    // Create event poller
    let events = EventPoller::new()?;

    // Get terminal dimensions
    let (cols, rows) = renderer.context().char_dimensions();

    // Create UI components
    let mut root = Container::column();

    // Header (fixed 1 row)
    let header = Box::new(Header::new());
    root.add_fixed(header, 1);

    // Main area (flex to fill remaining space)
    let main_area = Container::column();
    // TODO: Add actual content components here
    root.add_flex(Box::new(main_area), 1);

    // Status bar (fixed 1 row)
    let status = Box::new(StatusBar::with_text("Press 'q' to quit", "", &theme));
    root.add_fixed(status, 1);

    // Create slots and render context
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    // Main render loop
    loop {
        // Render UI
        let bounds = Rect::fullscreen(cols, rows);
        root.render(&mut renderer, bounds, &ctx)?;
        renderer.flush()?;

        // Poll for events with timeout
        if let Some(event) = events.poll(Duration::from_millis(16))? {
            match event {
                Event::Key(Key::Char('q')) | Event::Key(Key::Ctrl('c')) => {
                    break;
                }
                Event::Key(Key::Esc) => {
                    break;
                }
                Event::Resize(_cols, _rows) => {
                    renderer.refresh_geometry()?;
                    renderer.clear()?;
                    root.mark_dirty();
                }
                _ => {}
            }
        }
    }

    // Cleanup
    renderer.exit_alt_screen()?;
    renderer.show_cursor()?;

    println!("mkui demo finished");

    Ok(())
}

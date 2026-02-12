//! Reactive mkui demo - Shows terminal geometry with live updates

use anyhow::Result;
use mkui::{
    component::Component,
    components::{Container, Header, StatusBar, Text},
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

    let events = EventPoller::new()?;

    // Initial geometry
    let mut geom = renderer.context().geometry;
    let (mut cols, mut rows) = (geom.cols, geom.rows);

    // Build initial UI
    let mut root = build_ui(&theme, &geom);

    // Create slots and render context
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    // Main render loop
    loop {
        // Render UI
        let bounds = Rect::fullscreen(cols, rows);
        renderer.clear()?;
        root.render(&mut renderer, bounds, &ctx)?;
        renderer.flush()?;

        // Poll for events
        if let Some(event) = events.poll(Duration::from_millis(16))? {
            match event {
                Event::Key(Key::Char('q')) | Event::Key(Key::Ctrl('c')) | Event::Key(Key::Esc) => {
                    break;
                }
                Event::Resize(new_cols, new_rows) => {
                    // Update geometry
                    renderer.refresh_geometry()?;
                    geom = renderer.context().geometry;
                    cols = new_cols;
                    rows = new_rows;

                    // Rebuild UI with new geometry
                    root = build_ui(&theme, &geom);
                    root.mark_dirty();
                }
                _ => {}
            }
        }
    }

    // Cleanup
    renderer.exit_alt_screen()?;
    renderer.show_cursor()?;

    println!("mkui reactive demo finished");

    Ok(())
}

/// Build UI with current geometry
fn build_ui(theme: &Theme, geom: &mkui::TerminalGeometry) -> Container {
    let mut root = Container::column();

    // Header (fixed 1 row)
    let header = Box::new(Header::new());
    root.add_fixed(header, 1);

    // Main content area (flex to fill)
    let mut content = Container::column();

    // Add spacer to push content to center vertically
    content.add_flex(Box::new(Text::new("")), 1);

    // Create geometry display text (centered both horizontally and vertically)
    let geometry_text = format_geometry(geom);
    let text = Box::new(
        Text::new(geometry_text)
            .with_align(mkui::components::text::TextAlign::Center)
            .with_style(theme.text_style()),
    );
    content.add_fixed(text, 3); // Fixed height for the 3 lines of text

    // Add spacer below to center vertically
    content.add_flex(Box::new(Text::new("")), 1);

    root.add_flex(Box::new(content), 1);

    // Status bar (fixed 1 row)
    let status = Box::new(StatusBar::with_text(
        "Resize your terminal | Press 'q' to quit",
        "",
        theme,
    ));
    root.add_fixed(status, 1);

    root
}

/// Format geometry information for display
fn format_geometry(geom: &mkui::TerminalGeometry) -> String {
    let mut lines = Vec::new();

    // Character dimensions
    lines.push(format!("Size: {} cols × {} rows", geom.cols, geom.rows));

    // Pixel dimensions (if available)
    if let (Some(pw), Some(ph)) = (geom.pixel_width, geom.pixel_height) {
        lines.push(format!("Pixels: {} × {}", pw, ph));
        lines.push(format!(
            "Cell: {}px × {}px",
            geom.char_width, geom.char_height
        ));
    } else {
        lines.push(format!(
            "Estimated cells: {}px × {}px",
            geom.char_width, geom.char_height
        ));
    }

    // Join with newlines
    lines.join("\n")
}

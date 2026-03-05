use anyhow::Result;
use mkui::components::{Header, StatusBar, Text};
use mkui::context::RenderContext;
use mkui::event::{Event, EventPoller, Key};
use mkui::layout::{FlexDirection, FlexLayout, Rect, Size};
use mkui::render::Renderer;
use mkui::slots::Slots;
use mkui::terminal::TerminalCapabilities;
use mkui::theme::Theme;
use mkui::Component;

fn main() -> Result<()> {
    let mut renderer = Renderer::new()?;
    renderer.enter_alt_screen()?;

    let caps = TerminalCapabilities::detect();
    let theme = Theme::new(caps);
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    let events = EventPoller::new()?;

    let mut header = Header::new();
    let mut content = Text::new("Press 'q' or ESC to quit.");
    let mut status = StatusBar::with_text("mkui", "v0.1.0", &theme);

    loop {
        let (cols, rows) = renderer.context().char_dimensions();
        let bounds = Rect::fullscreen(cols, rows);

        let layout = FlexLayout::new(FlexDirection::Column);
        let rects = layout.layout(bounds, &[Size::Fixed(1), Size::Flex(1), Size::Fixed(1)]);

        renderer.begin_frame()?;
        renderer.clear()?;

        header.render(&mut renderer, rects[0], &ctx)?;
        content.render(&mut renderer, rects[1], &ctx)?;
        status.render(&mut renderer, rects[2], &ctx)?;

        renderer.end_frame()?;

        match events.read()? {
            Event::Key(Key::Char('q') | Key::Esc) => break,
            Event::Resize(_, _) => renderer.refresh_geometry()?,
            _ => {}
        }
    }

    Ok(())
}

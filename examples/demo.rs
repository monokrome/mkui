use anyhow::Result;
use mkui::app::App;
use mkui::components::{Header, StatusBar, Text};
use mkui::context::RenderContext;
use mkui::event::{Event, Key};
use mkui::layout::{FlexDirection, FlexLayout, Rect, Size};
use mkui::render::Renderer;
use mkui::slots::Slots;
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
                eprintln!("GUI support not compiled. Rebuild with the gui feature enabled.");
                std::process::exit(1);
            }
        }
        "--tui" => {
            #[cfg(feature = "tui")]
            {
                run_tui()?;
            }
            #[cfg(not(feature = "tui"))]
            {
                eprintln!("TUI support not compiled. Rebuild with the tui feature enabled.");
                std::process::exit(1);
            }
        }
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

#[cfg(feature = "tui")]
fn run_tui() -> Result<()> {
    let theme = Theme::new();
    let slots = Slots::new();
    let ctx = RenderContext::new(&theme, &slots);

    let mut header = Header::new();
    let mut content = Text::new("TUI mode. Press 'q' or ESC to quit.");
    let mut status = StatusBar::with_text("mkui", "TUI", &theme);

    App::run_tui(|event: &Event, renderer: &mut dyn Renderer| {
        let _ = render_frame(renderer, &mut header, &mut content, &mut status, &ctx);

        !event.is_key(Key::Char('q')) && !event.is_key(Key::Esc)
    })
}

#[cfg(feature = "gui")]
fn run_gui() -> Result<()> {
    let theme = Theme::new();

    let mut header = Header::new();
    let mut content = Text::new("GUI mode. Press 'q' or ESC to quit.");
    let mut status = StatusBar::with_text("mkui", "GUI", &theme);

    App::run_gui("mkui demo", 16.0, move |event: &Event, renderer: &mut dyn Renderer| {
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);
        let _ = render_frame(renderer, &mut header, &mut content, &mut status, &ctx);

        !event.is_key(Key::Char('q')) && !event.is_key(Key::Esc)
    })
}

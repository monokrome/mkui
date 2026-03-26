//! Panel component — titled container with optional border
//!
//! A panel renders a header bar with a title, then delegates the remaining
//! space to a content component. Useful for panes, sidebars, and overlays.

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler};
use crate::layout::Rect;
use crate::render::Renderer;
use crate::style::Style;
use crate::theme::Color;
use anyhow::Result;

/// A titled container that renders a header and delegates content
pub struct Panel {
    title: String,
    header_style: Style,
    border_style: Option<Style>,
    content: Box<dyn Component>,
}

impl Panel {
    /// Create a new panel with a title and content component
    pub fn new(title: impl Into<String>, content: Box<dyn Component>) -> Self {
        Panel {
            title: title.into(),
            header_style: Style::new().bold(true).reverse(true),
            border_style: None,
            content,
        }
    }

    /// Set the header style
    pub fn with_header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Set a colored header background
    pub fn with_header_colors(mut self, fg: Color, bg: Color) -> Self {
        self.header_style = Style::new().fg(fg).bg(bg).bold(true);
        self
    }

    /// Enable a border with the given style
    pub fn with_border(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    /// Get mutable access to the content component
    pub fn content_mut(&mut self) -> &mut dyn Component {
        &mut *self.content
    }

    /// Update the title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }
}

impl EventHandler for Panel {
    fn handle_event(&mut self, event: &Event) -> bool {
        self.content.handle_event(event)
    }
}

impl Component for Panel {
    fn render(
        &mut self,
        renderer: &mut dyn Renderer,
        bounds: Rect,
        ctx: &RenderContext,
    ) -> Result<()> {
        if bounds.height == 0 {
            return Ok(());
        }

        // Header (1 row)
        renderer.fill_rect(Rect::new(bounds.x, bounds.y, bounds.width, 1),
            self.header_style.bg.unwrap_or(Color::black()))?;
        renderer.move_cursor(bounds.x, bounds.y)?;

        let max_title = bounds.width as usize;
        let display_title = if self.title.len() > max_title {
            &self.title[..max_title]
        } else {
            &self.title
        };
        renderer.write_styled(display_title, &self.header_style)?;

        // Content area
        let content_bounds = Rect::new(
            bounds.x,
            bounds.y + 1,
            bounds.width,
            bounds.height.saturating_sub(1),
        );

        if content_bounds.height > 0 {
            self.content.render(renderer, content_bounds, ctx)?;
        }

        // Optional border
        if let Some(ref border_style) = self.border_style {
            let right = bounds.x + bounds.width.saturating_sub(1);
            for row in 0..bounds.height {
                renderer.move_cursor(right, bounds.y + row)?;
                renderer.write_styled("│", border_style)?;
            }
        }

        Ok(())
    }

    fn generation(&self) -> u64 {
        self.content.generation()
    }

    fn name(&self) -> &str {
        "Panel"
    }
}

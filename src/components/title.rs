//! Title component - centered text for headers

use crate::component::Component;
use crate::components::slotted_bar::SlotContent;
use crate::components::text::{Text, TextAlign};
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use crate::theme::Theme;
use anyhow::Result;

/// Title component - displays centered text with header styling.
/// Thin wrapper around `Text` with `TextAlign::Center` and theme-based styling.
/// Also implements `SlotContent` for use in slotted bars.
pub struct Title {
    inner: Text,
}

impl Title {
    /// Create new title with centered text
    pub fn new(content: impl Into<String>, theme: &Theme) -> Self {
        Title {
            inner: Text::new(content)
                .with_align(TextAlign::Center)
                .with_style(theme.header_title_style()),
        }
    }

    /// Update title text
    pub fn set_text(&mut self, content: impl Into<String>) {
        self.inner.set_text(content);
    }

    /// Get title text
    pub fn text(&self) -> &str {
        self.inner.text()
    }
}

impl EventHandler for Title {}

impl Component for Title {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        self.inner.render(renderer, bounds, ctx)
    }

    fn min_size(&self) -> (u16, u16) {
        self.inner.min_size()
    }

    fn mark_dirty(&mut self) {
        self.inner.mark_dirty();
    }

    fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }

    fn name(&self) -> &str {
        "Title"
    }
}

impl SlotContent for Title {
    fn responsive_sizes(&self) -> Vec<crate::components::slotted_bar::SlotSize> {
        use crate::components::slotted_bar::SlotSize;

        let text_len = self.inner.text().len() as u16;

        vec![
            SlotSize::Fill,
            SlotSize::Percent(50),
            SlotSize::Blocks(text_len),
        ]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

//! Logo component - styled badge with background color

use crate::component::Component;
use crate::components::slotted_bar::SlotContent;
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Logo component - displays text with white background and black text
/// Includes 1 character padding on left and right
pub struct Logo {
    text: String,
    dirty: bool,
}

impl Logo {
    /// Create new logo with text
    pub fn new(text: impl Into<String>) -> Self {
        Logo {
            text: text.into(),
            dirty: true,
        }
    }
}

impl EventHandler for Logo {}

impl Component for Logo {
    fn render(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        if self.text.is_empty() || bounds.width == 0 {
            return Ok(());
        }

        // Content with padding: " TEXT "
        let padded = format!(" {} ", self.text);
        let content_len = padded.len() as u16;

        // Right-align within bounds
        let x = bounds
            .x
            .saturating_add(bounds.width.saturating_sub(content_len));

        renderer.move_cursor(x, bounds.y)?;
        // White background (47), black text (30)
        renderer.write_styled(&padded, "\x1b[47;30m")?;
        // Reset after
        renderer.write_text("\x1b[0m")?;

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        // Text + 2 chars padding
        ((self.text.len() + 2) as u16, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "Logo"
    }
}

impl SlotContent for Logo {
    fn responsive_sizes(&self) -> Vec<crate::components::slotted_bar::SlotSize> {
        use crate::components::slotted_bar::SlotSize;
        // Fixed size - text + padding
        vec![SlotSize::Blocks((self.text.len() + 2) as u16)]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

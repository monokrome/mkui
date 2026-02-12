//! Built-in slot content implementations

use crate::component::Component;
use crate::components::slotted_bar::SlotContent;
use crate::components::text::TextAlign;
use crate::context::{RenderContext, UseTheme};
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Text slot content with alignment and styling
pub struct TextSlot {
    text: String,
    align: TextAlign,
    style: String,
    fixed_width: Option<u16>,
    dirty: bool,
}

impl TextSlot {
    /// Create a new text slot
    pub fn new(text: impl Into<String>) -> Self {
        TextSlot {
            text: text.into(),
            align: TextAlign::Start,
            style: String::new(),
            fixed_width: None,
            dirty: true,
        }
    }

    /// Set text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set text style
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    /// Set a fixed width (useful for badges, labels)
    pub fn with_fixed_width(mut self, width: u16) -> Self {
        self.fixed_width = Some(width);
        self
    }

    /// Update the text
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.dirty = true;
    }

    /// Get the text
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl EventHandler for TextSlot {}

impl Component for TextSlot {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        let text_len = self.text.len() as u16;

        // Don't render if no space
        if bounds.width == 0 {
            return Ok(());
        }

        // Truncate if needed (no overflow)
        let display_text = if text_len > bounds.width {
            &self.text[..bounds.width as usize]
        } else {
            &self.text
        };

        // Resolve logical alignment to physical based on text direction
        let text_direction = self.use_text_direction(ctx);
        let physical_align = self.align.resolve(text_direction);

        // Calculate x position based on resolved physical alignment
        let x = match physical_align {
            crate::components::text::PhysicalAlign::Left => bounds.x,
            crate::components::text::PhysicalAlign::Center => {
                let offset = (bounds.width.saturating_sub(display_text.len() as u16)) / 2;
                bounds.x.saturating_add(offset)
            }
            crate::components::text::PhysicalAlign::Right => {
                let offset = bounds.width.saturating_sub(display_text.len() as u16);
                bounds.x.saturating_add(offset)
            }
        };

        // Render
        renderer.move_cursor(x, bounds.y)?;
        if self.style.is_empty() {
            renderer.write_text(display_text)?;
        } else {
            renderer.write_styled(display_text, &self.style)?;
        }

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (self.text.len() as u16, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "TextSlot"
    }
}

impl SlotContent for TextSlot {
    fn responsive_sizes(&self) -> Vec<crate::components::slotted_bar::SlotSize> {
        use crate::components::slotted_bar::SlotSize;

        if let Some(fixed) = self.fixed_width {
            // Fixed width - only one size
            vec![SlotSize::Blocks(fixed)]
        } else {
            // Flexible - can fill or shrink to text length
            let text_len = self.text.len() as u16;
            vec![SlotSize::Fill, SlotSize::Blocks(text_len)]
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Badge slot - fixed width label with specific styling (like "BETA" or "v1.0")
pub struct Badge {
    text: String,
    style: String,
    padding: u16,
    dirty: bool,
}

impl Badge {
    /// Create a new badge
    pub fn new(text: impl Into<String>) -> Self {
        Badge {
            text: text.into(),
            style: "\x1b[7m".to_string(), // Default: inverse video
            padding: 1,
            dirty: true,
        }
    }

    /// Set the badge style
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    /// Set padding around the badge text
    pub fn with_padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Get total width (text + padding on both sides)
    fn total_width(&self) -> u16 {
        self.text.len() as u16 + (self.padding * 2)
    }
}

impl EventHandler for Badge {}

impl Component for Badge {
    fn render(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        // Render padded text with style
        let padding_str = " ".repeat(self.padding as usize);
        let full_text = format!("{}{}{}", padding_str, self.text, padding_str);

        renderer.move_cursor(bounds.x, bounds.y)?;
        renderer.write_styled(&full_text, &self.style)?;

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (self.total_width(), 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "Badge"
    }
}

impl SlotContent for Badge {
    fn responsive_sizes(&self) -> Vec<crate::components::slotted_bar::SlotSize> {
        use crate::components::slotted_bar::SlotSize;

        // Badge is fixed width - only one size
        vec![SlotSize::Blocks(self.total_width())]
    }

    fn can_hide(&self) -> bool {
        false // Badges typically shouldn't hide
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Flexible spacer that expands to fill available space
pub struct Spacer;

impl Spacer {
    pub fn new() -> Self {
        Spacer
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for Spacer {}

impl Component for Spacer {
    fn render(
        &mut self,
        _renderer: &mut Renderer,
        _bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        // Spacer doesn't render anything
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (0, 1)
    }

    fn mark_dirty(&mut self) {
        // Spacer has no state to dirty
    }

    fn is_dirty(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "Spacer"
    }
}

impl SlotContent for Spacer {
    fn responsive_sizes(&self) -> Vec<crate::components::slotted_bar::SlotSize> {
        use crate::components::slotted_bar::SlotSize;

        // Spacer just fills available space
        vec![SlotSize::Fill]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

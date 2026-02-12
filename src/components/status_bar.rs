//! Status bar component for bottom bar with info

use crate::component::Component;
use crate::components::slot_content::TextSlot;
use crate::components::slotted_bar::SlottedBar;
use crate::components::text::TextAlign;
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use crate::slots::status_slots;
use crate::theme::Theme;
use anyhow::Result;

/// Status bar component with message (left) and mode (right)
pub struct StatusBar {
    bar: SlottedBar,
    message_idx: usize,
    mode_idx: usize,
    /// If true, read content from context slots instead of stored values
    use_context_slots: bool,
}

impl StatusBar {
    /// Create new status bar with theme
    pub fn new(theme: &Theme) -> Self {
        Self::build("", "", false, theme)
    }

    /// Create a status bar that reads from context slots.
    /// The status bar will read from status_slots::MESSAGE (left) and MODE (right).
    pub fn from_context(theme: &Theme) -> Self {
        Self::build("", "", true, theme)
    }

    /// Create with initial text and theme
    pub fn with_text(message: impl Into<String>, mode: impl Into<String>, theme: &Theme) -> Self {
        Self::build(message, mode, false, theme)
    }

    fn build(
        message: impl Into<String>,
        mode: impl Into<String>,
        use_context_slots: bool,
        theme: &Theme,
    ) -> Self {
        let mut bar = SlottedBar::new().with_background(theme.status_bg_fill());

        let message_slot = TextSlot::new(message)
            .with_align(TextAlign::Start)
            .with_style(theme.status_style());
        bar.add(Box::new(message_slot), 50);

        let mode_slot = TextSlot::new(mode)
            .with_align(TextAlign::End)
            .with_style(theme.status_style());
        bar.add(Box::new(mode_slot), 50);

        StatusBar {
            bar,
            message_idx: 0,
            mode_idx: 1,
            use_context_slots,
        }
    }

    /// Update the slot contents from context if using context slots
    fn sync_from_context(&mut self, ctx: &RenderContext) {
        if !self.use_context_slots {
            return;
        }

        // Get slot content from context
        let message = ctx.slots.status.get_text(status_slots::MESSAGE);
        let mode = ctx.slots.status.get_text(status_slots::MODE);

        // Update bar slot content
        if let Some(slot) = self.bar.get_slot_mut(self.message_idx) {
            if let Some(text_slot) = (**slot).as_any_mut().downcast_mut::<TextSlot>() {
                text_slot.set_text(message);
            }
        }
        if let Some(slot) = self.bar.get_slot_mut(self.mode_idx) {
            if let Some(text_slot) = (**slot).as_any_mut().downcast_mut::<TextSlot>() {
                text_slot.set_text(mode);
            }
        }
    }
}

// Removed Default impl - now requires Theme

impl EventHandler for StatusBar {
    fn handle_event(&mut self, event: &crate::event::Event) -> bool {
        self.bar.handle_event(event)
    }
}

impl Component for StatusBar {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        // Sync slot content from context if using context slots
        self.sync_from_context(ctx);
        self.bar.render(renderer, bounds, ctx)
    }

    fn min_size(&self) -> (u16, u16) {
        (10, 1)
    }

    fn mark_dirty(&mut self) {
        self.bar.mark_dirty();
    }

    fn is_dirty(&self) -> bool {
        self.bar.is_dirty()
    }

    fn name(&self) -> &str {
        "StatusBar"
    }
}

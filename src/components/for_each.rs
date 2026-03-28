//! Iteration component for rendering collections of items
//!
//! `ForEach` renders a scrollable, virtualized list of items where each item's
//! appearance is controlled by a user-provided closure. Unlike `List`, it makes
//! no assumptions about how items look — you have full control per item.
//!
//! ```ignore
//! let files = vec![/* ... */];
//! let mut view = ForEach::new(files.len(), 1, |index, renderer, bounds, ctx| {
//!     let file = &files[index];
//!     renderer.move_cursor(bounds.x, bounds.y)?;
//!     renderer.write_text(&file.name)?;
//!     Ok(())
//! });
//! ```

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use crate::signal::Signal;
use crate::signal::SignalBase;
use anyhow::Result;

/// Render callback for a single item
///
/// Receives the item index, renderer, bounds for this item, and render context.
pub type ItemRenderer = Box<dyn FnMut(usize, &mut dyn Renderer, Rect, &RenderContext) -> Result<()>>;

/// Scrollable iteration component
///
/// Renders a virtualized list of items, calling a closure for each visible item.
/// Handles scroll offset, cursor position, and viewport management.
pub struct ForEach {
    /// Total number of items
    item_count: Signal<usize>,
    /// Height of each item in rows
    item_height: u16,
    /// Current cursor index
    cursor: Signal<usize>,
    /// Scroll offset (first visible item index)
    scroll_offset: usize,
    /// Whether this component has focus
    focused: bool,
    /// Render closure called for each visible item
    render_item: ItemRenderer,
    /// Scroll margin (rows to keep visible above/below cursor)
    scroll_margin: usize,
}

impl ForEach {
    /// Create a new ForEach with the given item count and per-item height
    pub fn new<F>(item_count: usize, item_height: u16, render_item: F) -> Self
    where
        F: FnMut(usize, &mut dyn Renderer, Rect, &RenderContext) -> Result<()> + 'static,
    {
        ForEach {
            item_count: Signal::new(item_count),
            item_height,
            cursor: Signal::new(0),
            scroll_offset: 0,
            focused: false,
            render_item: Box::new(render_item),
            scroll_margin: 2,
        }
    }

    /// Set the scroll margin (rows kept visible above/below cursor)
    pub fn with_scroll_margin(mut self, margin: usize) -> Self {
        self.scroll_margin = margin;
        self
    }

    /// Update the total item count
    pub fn set_item_count(&mut self, count: usize) {
        self.item_count.set(count);
        let cursor = *self.cursor.get();
        if cursor >= count && count > 0 {
            self.cursor.set(count - 1);
        }
    }

    /// Get the current cursor index
    pub fn cursor(&self) -> usize {
        *self.cursor.get()
    }

    /// Set the cursor to a specific index
    pub fn set_cursor(&mut self, index: usize) {
        let count = *self.item_count.get();
        if count > 0 {
            self.cursor.set(index.min(count - 1));
        }
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let count = *self.item_count.get();
        let cursor = *self.cursor.get();
        if cursor + 1 < count {
            self.cursor.set(cursor + 1);
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        let cursor = *self.cursor.get();
        if cursor > 0 {
            self.cursor.set(cursor - 1);
        }
    }

    /// Move cursor to first item
    pub fn cursor_first(&mut self) {
        self.cursor.set(0);
    }

    /// Move cursor to last item
    pub fn cursor_last(&mut self) {
        let count = *self.item_count.get();
        if count > 0 {
            self.cursor.set(count - 1);
        }
    }

    /// Move cursor down by a page
    pub fn page_down(&mut self, visible_rows: usize) {
        let count = *self.item_count.get();
        let cursor = *self.cursor.get();
        let page = visible_rows / self.item_height as usize;
        self.cursor.set((cursor + page).min(count.saturating_sub(1)));
    }

    /// Move cursor up by a page
    pub fn page_up(&mut self, visible_rows: usize) {
        let cursor = *self.cursor.get();
        let page = visible_rows / self.item_height as usize;
        self.cursor.set(cursor.saturating_sub(page));
    }

    /// Ensure the cursor is visible by adjusting scroll offset
    fn ensure_cursor_visible(&mut self, visible_items: usize) {
        let cursor = *self.cursor.get();

        if visible_items == 0 {
            return;
        }

        // Scroll down if cursor is below viewport
        let bottom_threshold = self.scroll_offset + visible_items.saturating_sub(self.scroll_margin + 1);
        if cursor > bottom_threshold {
            self.scroll_offset = cursor.saturating_sub(visible_items.saturating_sub(self.scroll_margin + 1));
        }

        // Scroll up if cursor is above viewport
        let top_threshold = self.scroll_offset + self.scroll_margin;
        if cursor < top_threshold {
            self.scroll_offset = cursor.saturating_sub(self.scroll_margin);
        }

        // Clamp scroll offset
        let count = *self.item_count.get();
        let max_offset = count.saturating_sub(visible_items);
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }
}

impl EventHandler for ForEach {
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.focused {
            return false;
        }

        let kind = &event.kind;

        if kind.is_key_press(Key::Char('j')) || kind.is_key_press(Key::Down) {
            self.cursor_down();
            true
        } else if kind.is_key_press(Key::Char('k')) || kind.is_key_press(Key::Up) {
            self.cursor_up();
            true
        } else if kind.is_key_press(Key::Char('g')) {
            self.cursor_first();
            true
        } else if kind.is_key_press(Key::Char('G')) {
            self.cursor_last();
            true
        } else if kind.is_ctrl('d') || kind.is_key_press(Key::PageDown) {
            self.page_down(20); // approximate; real value comes from render bounds
            true
        } else if kind.is_ctrl('u') || kind.is_key_press(Key::PageUp) {
            self.page_up(20);
            true
        } else {
            false
        }
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Component for ForEach {
    fn render(
        &mut self,
        renderer: &mut dyn Renderer,
        bounds: Rect,
        ctx: &RenderContext,
    ) -> Result<()> {
        let count = *self.item_count.get();
        if count == 0 {
            return Ok(());
        }

        let visible_items = bounds.height as usize / self.item_height as usize;
        self.ensure_cursor_visible(visible_items);

        let end = (self.scroll_offset + visible_items).min(count);

        for i in self.scroll_offset..end {
            let row_offset = (i - self.scroll_offset) as u16 * self.item_height;
            let item_bounds = Rect {
                x: bounds.x,
                y: bounds.y + row_offset,
                width: bounds.width,
                height: self.item_height,
            };

            (self.render_item)(i, renderer, item_bounds, ctx)?;
        }

        Ok(())
    }

    fn generation(&self) -> u64 {
        self.item_count.generation() + self.cursor.generation()
    }

    fn min_size(&self) -> (u16, u16) {
        (1, self.item_height)
    }

    fn name(&self) -> &str {
        "ForEach"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_navigation() {
        let mut view = ForEach::new(10, 1, |_, _, _, _| Ok(()));
        assert_eq!(view.cursor(), 0);

        view.cursor_down();
        assert_eq!(view.cursor(), 1);

        view.cursor_last();
        assert_eq!(view.cursor(), 9);

        view.cursor_down(); // shouldn't go past end
        assert_eq!(view.cursor(), 9);

        view.cursor_first();
        assert_eq!(view.cursor(), 0);

        view.cursor_up(); // shouldn't go below 0
        assert_eq!(view.cursor(), 0);
    }

    #[test]
    fn test_set_item_count_clamps_cursor() {
        let mut view = ForEach::new(10, 1, |_, _, _, _| Ok(()));
        view.set_cursor(9);
        assert_eq!(view.cursor(), 9);

        view.set_item_count(5);
        assert_eq!(view.cursor(), 4);
    }

    #[test]
    fn test_generation_changes_on_mutation() {
        let mut view = ForEach::new(10, 1, |_, _, _, _| Ok(()));
        let gen1 = view.generation();

        view.cursor_down();
        let gen2 = view.generation();
        assert!(gen2 > gen1);

        view.set_item_count(20);
        let gen3 = view.generation();
        assert!(gen3 > gen2);
    }

    #[test]
    fn test_scroll_offset() {
        let mut view = ForEach::new(100, 1, |_, _, _, _| Ok(())).with_scroll_margin(0);

        // Move to item 30, viewport of 10
        view.set_cursor(30);
        view.ensure_cursor_visible(10);

        assert!(view.scroll_offset <= 30);
        assert!(view.scroll_offset + 10 > 30);
    }
}

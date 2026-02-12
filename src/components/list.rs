//! Generic list component with Vim-style navigation
//!
//! Provides a navigable list with:
//! - j/k navigation
//! - Selection highlighting
//! - Virtual scrolling for large lists
//! - Focus integration
//!
//! # Example
//!
//! ```ignore
//! let items = vec!["Item 1", "Item 2", "Item 3"];
//! let mut list = List::new(items);
//! list.select(0);
//!
//! // In event handler:
//! match key {
//!     Key::Char('j') => list.select_next(),
//!     Key::Char('k') => list.select_prev(),
//!     Key::Enter => {
//!         if let Some(item) = list.selected() {
//!             // Handle selection
//!         }
//!     }
//!     _ => {}
//! }
//! ```

use crate::component::Component;
use crate::components::scrollable::ScrollableView;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Selection mode for the list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Single item selection (default)
    #[default]
    Single,
    /// Multiple items can be selected
    Multiple,
    /// No selection allowed (display only)
    None,
}

/// List item renderer function type
/// Parameters: item, is_selected, width
pub type ItemRenderer<T> = Box<dyn Fn(&T, bool, u16) -> String>;

/// Generic list component
///
/// A navigable list with virtual scrolling support for large datasets.
#[derive(Debug)]
pub struct List<T> {
    /// Items in the list
    items: Vec<T>,

    /// Currently selected index (None if no selection)
    selected_index: Option<usize>,

    /// Multiple selection indices (for SelectionMode::Multiple)
    selected_indices: Vec<usize>,

    /// Selection mode
    selection_mode: SelectionMode,

    /// Scroll state
    scroll: ScrollableView,

    /// Whether this component has focus
    focused: bool,

    /// Whether the component needs redraw
    dirty: bool,

    /// Viewport height (set during render)
    viewport_height: u16,
}

impl<T> List<T> {
    /// Create a new list with the given items
    pub fn new(items: Vec<T>) -> Self {
        let height = items.len();
        Self {
            items,
            selected_index: None,
            selected_indices: Vec::new(),
            selection_mode: SelectionMode::Single,
            scroll: ScrollableView::vertical(height),
            focused: false,
            dirty: true,
            viewport_height: 10,
        }
    }

    /// Create an empty list
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Set selection mode
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Set items, resetting selection
    pub fn set_items(&mut self, items: Vec<T>) {
        let height = items.len();
        self.items = items;
        self.selected_index = None;
        self.selected_indices.clear();
        self.scroll = ScrollableView::vertical(height);
        self.dirty = true;
    }

    /// Get items
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Get mutable items
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        self.dirty = true;
        &mut self.items
    }

    /// Get number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get currently selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Get currently selected item
    pub fn selected(&self) -> Option<&T> {
        self.selected_index.and_then(|i| self.items.get(i))
    }

    /// Get mutable reference to selected item
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        self.dirty = true;
        self.selected_index.and_then(|i| self.items.get_mut(i))
    }

    /// Select an item by index
    pub fn select(&mut self, index: usize) -> bool {
        if index < self.items.len() {
            match self.selection_mode {
                SelectionMode::Single => {
                    self.selected_index = Some(index);
                }
                SelectionMode::Multiple => {
                    if !self.selected_indices.contains(&index) {
                        self.selected_indices.push(index);
                    }
                    self.selected_index = Some(index);
                }
                SelectionMode::None => return false,
            }
            self.ensure_visible(index);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Toggle selection for an item (multiple mode only)
    pub fn toggle_select(&mut self, index: usize) {
        if self.selection_mode != SelectionMode::Multiple {
            return;
        }
        if let Some(pos) = self.selected_indices.iter().position(|&i| i == index) {
            self.selected_indices.remove(pos);
        } else if index < self.items.len() {
            self.selected_indices.push(index);
        }
        self.dirty = true;
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
        self.selected_indices.clear();
        self.dirty = true;
    }

    /// Select the next item
    pub fn select_next(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let new_index = match self.selected_index {
            Some(i) if i + 1 < self.items.len() => i + 1,
            Some(i) => i, // Stay at end
            None => 0,    // Select first
        };
        self.select(new_index)
    }

    /// Select the previous item
    pub fn select_prev(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let new_index = match self.selected_index {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,                               // Stay at start
            None => self.items.len().saturating_sub(1), // Select last
        };
        self.select(new_index)
    }

    /// Select the first item
    pub fn select_first(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.select(0)
    }

    /// Select the last item
    pub fn select_last(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        self.select(self.items.len() - 1)
    }

    /// Move selection down by a page
    pub fn page_down(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let page_size = self.viewport_height.max(1) as usize;
        let new_index = self
            .selected_index
            .map(|i| (i + page_size).min(self.items.len() - 1))
            .unwrap_or(0);
        self.select(new_index)
    }

    /// Move selection up by a page
    pub fn page_up(&mut self) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let page_size = self.viewport_height.max(1) as usize;
        let new_index = self
            .selected_index
            .map(|i| i.saturating_sub(page_size))
            .unwrap_or(0);
        self.select(new_index)
    }

    /// Ensure an index is visible in the viewport
    fn ensure_visible(&mut self, index: usize) {
        self.scroll
            .ensure_visible(0, index, 1, self.viewport_height as usize);
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll.offset_y()
    }

    /// Check if an index is selected (for multiple selection mode)
    pub fn is_selected(&self, index: usize) -> bool {
        match self.selection_mode {
            SelectionMode::Single => self.selected_index == Some(index),
            SelectionMode::Multiple => self.selected_indices.contains(&index),
            SelectionMode::None => false,
        }
    }

    /// Get all selected indices (for multiple selection mode)
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.dirty = true;
        }
    }

    /// Check if focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Filter items (creates a new filtered list)
    pub fn filter<F>(&self, predicate: F) -> Vec<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().filter(|item| predicate(item)).collect()
    }
}

impl<T: ToString> List<T> {
    /// Render the list with default string conversion
    pub fn render_default(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        self.viewport_height = bounds.height;

        if self.items.is_empty() {
            renderer.move_cursor(bounds.x, bounds.y)?;
            renderer.write_text("(empty)")?;
            return Ok(());
        }

        let offset = self.scroll.offset_y();
        let visible_count = bounds.height as usize;

        for (i, item) in self
            .items
            .iter()
            .enumerate()
            .skip(offset)
            .take(visible_count)
        {
            let y = bounds.y + (i - offset) as u16;
            let is_selected = self.is_selected(i);
            let is_cursor = self.selected_index == Some(i);

            renderer.move_cursor(bounds.x, y)?;

            // Render item text, truncated to fit
            let text = item.to_string();
            let max_width = bounds.width as usize;
            let display_text = if text.len() > max_width {
                format!("{}...", &text[..max_width.saturating_sub(3)])
            } else {
                format!("{:width$}", text, width = max_width)
            };

            // Highlight selected items with ANSI colors
            if is_selected && self.focused {
                // Inverted colors for selection
                let style = "\x1b[7m".to_string(); // Reverse video
                renderer.write_styled(&display_text, &style)?;
            } else if is_cursor {
                // Underline for cursor
                let style = "\x1b[4m".to_string(); // Underline
                renderer.write_styled(&display_text, &style)?;
            } else {
                renderer.write_text(&display_text)?;
            }
        }

        self.dirty = false;
        Ok(())
    }
}

impl<T: ToString + 'static> EventHandler for List<T> {
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.focused {
            return false;
        }

        match event {
            Event::Key(key) => match key {
                Key::Char('j') | Key::Down => {
                    self.select_next();
                    true
                }
                Key::Char('k') | Key::Up => {
                    self.select_prev();
                    true
                }
                Key::Char('g') => {
                    self.select_first();
                    true
                }
                Key::Char('G') => {
                    self.select_last();
                    true
                }
                Key::Ctrl('d') | Key::PageDown => {
                    self.page_down();
                    true
                }
                Key::Ctrl('u') | Key::PageUp => {
                    self.page_up();
                    true
                }
                Key::Char(' ') if self.selection_mode == SelectionMode::Multiple => {
                    if let Some(idx) = self.selected_index {
                        self.toggle_select(idx);
                    }
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }
}

impl<T: ToString + 'static> Component for List<T> {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        self.render_default(renderer, bounds, ctx)
    }

    fn min_size(&self) -> (u16, u16) {
        (10, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "List"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let mut list = List::new(vec!["a", "b", "c"]);

        assert_eq!(list.selected(), None);

        list.select(1);
        assert_eq!(list.selected(), Some(&"b"));
        assert_eq!(list.selected_index(), Some(1));
    }

    #[test]
    fn test_navigation() {
        let mut list = List::new(vec!["a", "b", "c"]);

        list.select(0);
        assert_eq!(list.selected(), Some(&"a"));

        list.select_next();
        assert_eq!(list.selected(), Some(&"b"));

        list.select_next();
        assert_eq!(list.selected(), Some(&"c"));

        list.select_next(); // Should stay at end
        assert_eq!(list.selected(), Some(&"c"));

        list.select_prev();
        assert_eq!(list.selected(), Some(&"b"));
    }

    #[test]
    fn test_first_last() {
        let mut list = List::new(vec!["a", "b", "c", "d"]);

        list.select_last();
        assert_eq!(list.selected(), Some(&"d"));

        list.select_first();
        assert_eq!(list.selected(), Some(&"a"));
    }

    #[test]
    fn test_multiple_selection() {
        let mut list = List::new(vec!["a", "b", "c"]).with_selection_mode(SelectionMode::Multiple);

        list.select(0);
        list.toggle_select(1);
        list.toggle_select(2);

        assert!(list.is_selected(0));
        assert!(list.is_selected(1));
        assert!(list.is_selected(2));

        list.toggle_select(1);
        assert!(!list.is_selected(1));
    }

    #[test]
    fn test_empty_list() {
        let mut list: List<String> = List::empty();

        assert!(list.is_empty());
        assert!(!list.select_next());
        assert!(!list.select_prev());
        assert_eq!(list.selected(), None);
    }
}

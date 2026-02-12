//! Split view component for horizontal/vertical panes
//!
//! Provides Vim-style split panes with:
//! - Horizontal and vertical splits
//! - Resizable dividers
//! - Active pane tracking
//! - Ctrl-w navigation
//!
//! # Example
//!
//! ```ignore
//! let mut split = SplitView::new(Box::new(editor1));
//! split.split_vertical(Box::new(editor2));
//! split.split_horizontal(Box::new(browser));
//!
//! // Navigate between panes
//! split.focus_next();  // Ctrl-w w
//! split.focus_left();  // Ctrl-w h
//! split.focus_right(); // Ctrl-w l
//! ```

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    /// Split horizontally (panes side by side)
    #[default]
    Horizontal,
    /// Split vertically (panes stacked)
    Vertical,
}

/// A pane in the split view
pub struct Pane {
    /// Content component
    pub content: Box<dyn Component>,
    /// Minimum size in cells
    pub min_size: u16,
    /// Whether this pane is focused
    pub focused: bool,
}

impl Pane {
    /// Create a new pane with content
    pub fn new(content: Box<dyn Component>) -> Self {
        Self {
            content,
            min_size: 1,
            focused: false,
        }
    }

    /// Set minimum size
    pub fn with_min_size(mut self, size: u16) -> Self {
        self.min_size = size;
        self
    }
}

impl std::fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pane")
            .field("content", &self.content.name())
            .field("min_size", &self.min_size)
            .field("focused", &self.focused)
            .finish()
    }
}

/// Split view with multiple panes
#[derive(Debug)]
pub struct SplitView {
    /// Direction of the split
    direction: SplitDirection,
    /// Panes in this split
    panes: Vec<Pane>,
    /// Index of active pane
    active_pane: usize,
    /// Divider positions (ratios 0.0-1.0)
    divider_positions: Vec<f32>,
    /// Whether the split view is dirty
    dirty: bool,
    /// Divider character
    divider_char: char,
}

impl Default for SplitView {
    fn default() -> Self {
        Self {
            direction: SplitDirection::Horizontal,
            panes: Vec::new(),
            active_pane: 0,
            divider_positions: Vec::new(),
            dirty: true,
            divider_char: '│',
        }
    }
}

impl SplitView {
    /// Create a new split view with a single pane
    pub fn new(content: Box<dyn Component>) -> Self {
        Self {
            direction: SplitDirection::Horizontal,
            panes: vec![Pane::new(content)],
            active_pane: 0,
            divider_positions: Vec::new(),
            dirty: true,
            divider_char: '│',
        }
    }

    /// Create an empty split view
    pub fn empty() -> Self {
        Self::default()
    }

    /// Set split direction
    pub fn with_direction(mut self, direction: SplitDirection) -> Self {
        self.direction = direction;
        self.divider_char = match direction {
            SplitDirection::Horizontal => '│',
            SplitDirection::Vertical => '─',
        };
        self
    }

    /// Get current split direction
    pub fn direction(&self) -> SplitDirection {
        self.direction
    }

    /// Get number of panes
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get active pane index
    pub fn active_pane(&self) -> usize {
        self.active_pane
    }

    /// Check if there are multiple panes
    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// Add a pane with horizontal split (side by side)
    pub fn split_horizontal(&mut self, content: Box<dyn Component>) {
        self.split(content, SplitDirection::Horizontal);
    }

    /// Add a pane with vertical split (stacked)
    pub fn split_vertical(&mut self, content: Box<dyn Component>) {
        self.split(content, SplitDirection::Vertical);
    }

    /// Add a pane with the given split direction
    fn split(&mut self, content: Box<dyn Component>, direction: SplitDirection) {
        if self.panes.is_empty() {
            self.panes.push(Pane::new(content));
        } else {
            let insert_pos = self.active_pane + 1;
            self.panes.insert(insert_pos, Pane::new(content));
            self.recalculate_dividers();
            self.active_pane = insert_pos;
        }
        self.direction = direction;
        self.divider_char = match direction {
            SplitDirection::Horizontal => '│',
            SplitDirection::Vertical => '─',
        };
        self.dirty = true;
    }

    /// Close a pane by index
    pub fn close_pane(&mut self, index: usize) -> Option<Box<dyn Component>> {
        if index >= self.panes.len() || self.panes.len() <= 1 {
            return None;
        }

        let pane = self.panes.remove(index);

        // Recalculate dividers
        self.recalculate_dividers();

        // Adjust active pane
        if self.active_pane >= self.panes.len() {
            self.active_pane = self.panes.len() - 1;
        }

        self.dirty = true;
        Some(pane.content)
    }

    /// Close the active pane
    pub fn close_active(&mut self) -> Option<Box<dyn Component>> {
        self.close_pane(self.active_pane)
    }

    /// Recalculate divider positions to be evenly spaced
    fn recalculate_dividers(&mut self) {
        let count = self.panes.len();
        if count <= 1 {
            self.divider_positions.clear();
        } else {
            self.divider_positions = (1..count).map(|i| i as f32 / count as f32).collect();
        }
    }

    /// Resize a divider
    pub fn resize_divider(&mut self, index: usize, ratio: f32) {
        if index < self.divider_positions.len() {
            let clamped = ratio.clamp(0.1, 0.9);
            self.divider_positions[index] = clamped;
            self.dirty = true;
        }
    }

    /// Focus the next pane (Ctrl-w w)
    pub fn focus_next(&mut self) {
        if !self.panes.is_empty() {
            self.panes[self.active_pane].focused = false;
            self.active_pane = (self.active_pane + 1) % self.panes.len();
            self.panes[self.active_pane].focused = true;
            self.dirty = true;
        }
    }

    /// Focus the previous pane (Ctrl-w W)
    pub fn focus_prev(&mut self) {
        if !self.panes.is_empty() {
            self.panes[self.active_pane].focused = false;
            self.active_pane = if self.active_pane == 0 {
                self.panes.len() - 1
            } else {
                self.active_pane - 1
            };
            self.panes[self.active_pane].focused = true;
            self.dirty = true;
        }
    }

    /// Focus pane to the left (Ctrl-w h)
    pub fn focus_left(&mut self) {
        self.focus_directional(SplitDirection::Horizontal, -1);
    }

    /// Focus pane to the right (Ctrl-w l)
    pub fn focus_right(&mut self) {
        self.focus_directional(SplitDirection::Horizontal, 1);
    }

    /// Focus pane above (Ctrl-w k)
    pub fn focus_up(&mut self) {
        self.focus_directional(SplitDirection::Vertical, -1);
    }

    /// Focus pane below (Ctrl-w j)
    pub fn focus_down(&mut self) {
        self.focus_directional(SplitDirection::Vertical, 1);
    }

    fn focus_directional(&mut self, axis: SplitDirection, offset: isize) {
        if self.direction != axis {
            return;
        }

        let new_idx = self.active_pane as isize + offset;
        if new_idx >= 0 && (new_idx as usize) < self.panes.len() {
            self.panes[self.active_pane].focused = false;
            self.active_pane = new_idx as usize;
            self.panes[self.active_pane].focused = true;
            self.dirty = true;
        }
    }

    /// Focus a specific pane by index
    pub fn focus_pane(&mut self, index: usize) -> bool {
        if index < self.panes.len() {
            self.panes[self.active_pane].focused = false;
            self.active_pane = index;
            self.panes[self.active_pane].focused = true;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Get mutable reference to active pane content
    pub fn active_content_mut(&mut self) -> Option<&mut Box<dyn Component>> {
        self.panes.get_mut(self.active_pane).map(|p| &mut p.content)
    }

    /// Get reference to active pane content
    pub fn active_content(&self) -> Option<&dyn Component> {
        self.panes.get(self.active_pane).map(|p| &*p.content)
    }

    /// Calculate bounds for each pane
    fn calculate_pane_bounds(&self, bounds: Rect) -> Vec<Rect> {
        if self.panes.is_empty() {
            return vec![];
        }

        if self.panes.len() == 1 {
            return vec![bounds];
        }

        let is_horizontal = self.direction == SplitDirection::Horizontal;
        let total_main = if is_horizontal {
            bounds.width
        } else {
            bounds.height
        } as f32;

        let mut result = Vec::with_capacity(self.panes.len());
        let mut main_offset = 0u16;

        for i in 0..self.panes.len() {
            let end_ratio = self.divider_positions.get(i).copied().unwrap_or(1.0);
            let start_ratio = if i == 0 {
                0.0
            } else {
                self.divider_positions[i - 1]
            };

            let span = ((end_ratio - start_ratio) * total_main) as u16;
            let actual_span = if i < self.panes.len() - 1 {
                span.saturating_sub(1)
            } else {
                span
            };

            let rect = if is_horizontal {
                Rect::new(
                    bounds.x.saturating_add(main_offset),
                    bounds.y,
                    actual_span,
                    bounds.height,
                )
            } else {
                Rect::new(
                    bounds.x,
                    bounds.y.saturating_add(main_offset),
                    bounds.width,
                    actual_span,
                )
            };

            result.push(rect);
            main_offset += actual_span + 1;
        }

        result
    }
}

impl EventHandler for SplitView {
    fn handle_event(&mut self, event: &Event) -> bool {
        // First, try to handle in active pane
        if let Some(pane) = self.panes.get_mut(self.active_pane) {
            if pane.content.handle_event(event) {
                return true;
            }
        }

        // Handle split-level navigation (would normally be handled by app)
        match event {
            Event::Key(Key::Ctrl('w')) => {
                // This would typically be handled at app level
                // Just return false to let the app handle Ctrl-w commands
                false
            }
            _ => false,
        }
    }
}

impl Component for SplitView {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        if self.panes.is_empty() {
            return Ok(());
        }

        let pane_bounds = self.calculate_pane_bounds(bounds);
        let pane_count = self.panes.len();

        // Render each pane
        for (i, (pane, pane_rect)) in self.panes.iter_mut().zip(pane_bounds.iter()).enumerate() {
            pane.content.render(renderer, *pane_rect, ctx)?;

            // Draw divider after each pane (except last)
            if i < pane_count - 1 {
                match self.direction {
                    SplitDirection::Horizontal => {
                        let divider_x = pane_rect.x + pane_rect.width;
                        for y in pane_rect.y..pane_rect.y + pane_rect.height {
                            renderer.move_cursor(divider_x, y)?;
                            renderer.write_text(&self.divider_char.to_string())?;
                        }
                    }
                    SplitDirection::Vertical => {
                        let divider_y = pane_rect.y + pane_rect.height;
                        renderer.move_cursor(bounds.x, divider_y)?;
                        for _ in 0..bounds.width {
                            renderer.write_text(&self.divider_char.to_string())?;
                        }
                    }
                }
            }
        }

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        if self.panes.is_empty() {
            return (0, 0);
        }

        match self.direction {
            SplitDirection::Horizontal => {
                let total_width: u16 = self.panes.iter().map(|p| p.min_size).sum();
                let dividers = (self.panes.len().saturating_sub(1)) as u16;
                (total_width + dividers, 1)
            }
            SplitDirection::Vertical => {
                let total_height: u16 = self.panes.iter().map(|p| p.min_size).sum();
                let dividers = (self.panes.len().saturating_sub(1)) as u16;
                (1, total_height + dividers)
            }
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        for pane in &mut self.panes {
            pane.content.mark_dirty();
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.panes.iter().any(|p| p.content.is_dirty())
    }

    fn name(&self) -> &str {
        "SplitView"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test component
    struct TestPane {
        name: String,
    }

    impl EventHandler for TestPane {}

    impl Component for TestPane {
        fn render(
            &mut self,
            _renderer: &mut Renderer,
            _bounds: Rect,
            _ctx: &RenderContext,
        ) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    fn make_pane(name: &str) -> Box<dyn Component> {
        Box::new(TestPane {
            name: name.to_string(),
        })
    }

    #[test]
    fn test_single_pane() {
        let split = SplitView::new(make_pane("test"));
        assert_eq!(split.pane_count(), 1);
        assert!(!split.is_split());
    }

    #[test]
    fn test_horizontal_split() {
        let mut split = SplitView::new(make_pane("left"));
        split.split_horizontal(make_pane("right"));

        assert_eq!(split.pane_count(), 2);
        assert!(split.is_split());
        assert_eq!(split.active_pane(), 1); // New pane is focused
    }

    #[test]
    fn test_focus_navigation() {
        let mut split = SplitView::new(make_pane("a"));
        split.split_horizontal(make_pane("b"));
        split.split_horizontal(make_pane("c"));

        assert_eq!(split.active_pane(), 2);

        split.focus_prev();
        assert_eq!(split.active_pane(), 1);

        split.focus_next();
        assert_eq!(split.active_pane(), 2);

        split.focus_next(); // Wraps
        assert_eq!(split.active_pane(), 0);
    }

    #[test]
    fn test_close_pane() {
        let mut split = SplitView::new(make_pane("a"));
        split.split_horizontal(make_pane("b"));
        split.split_horizontal(make_pane("c"));

        assert_eq!(split.pane_count(), 3);

        split.close_pane(1);
        assert_eq!(split.pane_count(), 2);
    }

    #[test]
    fn test_bounds_calculation() {
        let mut split = SplitView::new(make_pane("a"));
        split.split_horizontal(make_pane("b"));

        let bounds = Rect::new(0, 0, 80, 24);
        let pane_bounds = split.calculate_pane_bounds(bounds);

        assert_eq!(pane_bounds.len(), 2);
        // Should roughly split the width in half (minus divider)
        assert!(pane_bounds[0].width > 35);
        assert!(pane_bounds[1].width > 35);
    }
}

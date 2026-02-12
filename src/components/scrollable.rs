//! Scrollable view component for viewport management
//!
//! Provides a viewport into larger content with:
//! - 2D scroll position tracking
//! - Visible range calculation
//! - Scroll-to and ensure-visible operations
//! - Page-based and line-based scrolling

use std::ops::Range;

/// Scrollable viewport manager
///
/// Manages scroll position and provides utilities for calculating
/// visible ranges of content. This is a pure data structure that
/// doesn't render anything itself - it's meant to be used by
/// components that need scrolling behavior.
///
/// # Example
/// ```ignore
/// let mut scroll = ScrollableView::new(1000, 500); // content: 1000x500
/// scroll.scroll_to(100, 0);
///
/// // Get visible range for a 80x24 viewport
/// let (x_range, y_range) = scroll.visible_range(80, 24);
/// // x_range = 100..180, y_range = 0..24
///
/// // Only render items within these ranges
/// for y in y_range {
///     for x in x_range.clone() {
///         // render item at (x, y)
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ScrollableView {
    /// Total content width (in content units - e.g., characters, pixels, items)
    content_width: usize,
    /// Total content height
    content_height: usize,
    /// Current horizontal scroll offset
    offset_x: usize,
    /// Current vertical scroll offset
    offset_y: usize,
    /// Whether to scroll by page (true) or by line (false) for large movements
    scroll_by_page: bool,
    /// Margin to keep around cursor when using ensure_visible
    scroll_margin: usize,
}

impl ScrollableView {
    /// Create a new scrollable view with given content dimensions
    pub fn new(content_width: usize, content_height: usize) -> Self {
        ScrollableView {
            content_width,
            content_height,
            offset_x: 0,
            offset_y: 0,
            scroll_by_page: false,
            scroll_margin: 3,
        }
    }

    /// Create a scrollable view for vertical-only scrolling
    pub fn vertical(content_height: usize) -> Self {
        ScrollableView::new(0, content_height)
    }

    /// Create a scrollable view for horizontal-only scrolling
    pub fn horizontal(content_width: usize) -> Self {
        ScrollableView::new(content_width, 0)
    }

    /// Enable or disable page-based scrolling
    pub fn with_page_scroll(mut self, enabled: bool) -> Self {
        self.scroll_by_page = enabled;
        self
    }

    /// Set scroll margin for ensure_visible
    pub fn with_scroll_margin(mut self, margin: usize) -> Self {
        self.scroll_margin = margin;
        self
    }

    /// Get current scroll offset
    pub fn offset(&self) -> (usize, usize) {
        (self.offset_x, self.offset_y)
    }

    /// Get horizontal offset
    pub fn offset_x(&self) -> usize {
        self.offset_x
    }

    /// Get vertical offset
    pub fn offset_y(&self) -> usize {
        self.offset_y
    }

    /// Get content dimensions
    pub fn content_size(&self) -> (usize, usize) {
        (self.content_width, self.content_height)
    }

    /// Update content dimensions
    pub fn set_content_size(&mut self, width: usize, height: usize) {
        self.content_width = width;
        self.content_height = height;
        // Clamp current offset to new bounds
        self.clamp_offset(width, height);
    }

    /// Scroll to absolute position
    pub fn scroll_to(&mut self, x: usize, y: usize) {
        self.offset_x = x.min(self.content_width);
        self.offset_y = y.min(self.content_height);
    }

    /// Scroll to horizontal position only
    pub fn scroll_to_x(&mut self, x: usize) {
        self.offset_x = x.min(self.content_width);
    }

    /// Scroll to vertical position only
    pub fn scroll_to_y(&mut self, y: usize) {
        self.offset_y = y.min(self.content_height);
    }

    /// Scroll by relative amount
    pub fn scroll_by(&mut self, dx: isize, dy: isize) {
        let new_x = if dx < 0 {
            self.offset_x.saturating_sub((-dx) as usize)
        } else {
            self.offset_x.saturating_add(dx as usize)
        };

        let new_y = if dy < 0 {
            self.offset_y.saturating_sub((-dy) as usize)
        } else {
            self.offset_y.saturating_add(dy as usize)
        };

        self.offset_x = new_x.min(self.content_width);
        self.offset_y = new_y.min(self.content_height);
    }

    /// Scroll up by one line or page
    pub fn scroll_up(&mut self, viewport_height: usize) {
        let amount = if self.scroll_by_page {
            viewport_height.saturating_sub(1)
        } else {
            1
        };
        self.offset_y = self.offset_y.saturating_sub(amount);
    }

    /// Scroll down by one line or page
    pub fn scroll_down(&mut self, viewport_height: usize) {
        let amount = if self.scroll_by_page {
            viewport_height.saturating_sub(1)
        } else {
            1
        };
        self.offset_y = self
            .offset_y
            .saturating_add(amount)
            .min(self.content_height);
    }

    /// Scroll left by one column or page
    pub fn scroll_left(&mut self, viewport_width: usize) {
        let amount = if self.scroll_by_page {
            viewport_width.saturating_sub(1)
        } else {
            1
        };
        self.offset_x = self.offset_x.saturating_sub(amount);
    }

    /// Scroll right by one column or page
    pub fn scroll_right(&mut self, viewport_width: usize) {
        let amount = if self.scroll_by_page {
            viewport_width.saturating_sub(1)
        } else {
            1
        };
        self.offset_x = self.offset_x.saturating_add(amount).min(self.content_width);
    }

    /// Page up (scroll up by viewport height)
    pub fn page_up(&mut self, viewport_height: usize) {
        self.offset_y = self
            .offset_y
            .saturating_sub(viewport_height.saturating_sub(1));
    }

    /// Page down (scroll down by viewport height)
    pub fn page_down(&mut self, viewport_height: usize) {
        self.offset_y = self
            .offset_y
            .saturating_add(viewport_height.saturating_sub(1))
            .min(self.content_height.saturating_sub(viewport_height));
    }

    /// Half page up
    pub fn half_page_up(&mut self, viewport_height: usize) {
        self.offset_y = self.offset_y.saturating_sub(viewport_height / 2);
    }

    /// Half page down
    pub fn half_page_down(&mut self, viewport_height: usize) {
        self.offset_y = self
            .offset_y
            .saturating_add(viewport_height / 2)
            .min(self.content_height.saturating_sub(viewport_height));
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.offset_y = 0;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        self.offset_y = self.content_height.saturating_sub(viewport_height);
    }

    /// Scroll to left edge
    pub fn scroll_to_left(&mut self) {
        self.offset_x = 0;
    }

    /// Scroll to right edge
    pub fn scroll_to_right(&mut self, viewport_width: usize) {
        self.offset_x = self.content_width.saturating_sub(viewport_width);
    }

    /// Ensure a position is visible within the viewport
    ///
    /// Scrolls the minimum amount necessary to make the position visible,
    /// respecting the scroll margin setting.
    pub fn ensure_visible(
        &mut self,
        x: usize,
        y: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) {
        let margin = self.scroll_margin;

        // Horizontal scrolling
        if viewport_width > 0 {
            let visible_start = self.offset_x + margin;
            let visible_end = self.offset_x + viewport_width.saturating_sub(margin);

            if x < visible_start {
                self.offset_x = x.saturating_sub(margin);
            } else if x >= visible_end && viewport_width > margin * 2 {
                self.offset_x =
                    x.saturating_sub(viewport_width.saturating_sub(margin).saturating_sub(1));
            }
        }

        // Vertical scrolling
        if viewport_height > 0 {
            let visible_start = self.offset_y + margin;
            let visible_end = self.offset_y + viewport_height.saturating_sub(margin);

            if y < visible_start {
                self.offset_y = y.saturating_sub(margin);
            } else if y >= visible_end && viewport_height > margin * 2 {
                self.offset_y =
                    y.saturating_sub(viewport_height.saturating_sub(margin).saturating_sub(1));
            }
        }

        // Clamp to valid range
        self.clamp_offset(viewport_width, viewport_height);
    }

    /// Center the viewport on a position
    pub fn center_on(&mut self, x: usize, y: usize, viewport_width: usize, viewport_height: usize) {
        self.offset_x = x.saturating_sub(viewport_width / 2);
        self.offset_y = y.saturating_sub(viewport_height / 2);
        self.clamp_offset(viewport_width, viewport_height);
    }

    /// Get the visible range of content coordinates
    ///
    /// Returns ranges that can be used to iterate over visible content.
    pub fn visible_range(
        &self,
        viewport_width: usize,
        viewport_height: usize,
    ) -> (Range<usize>, Range<usize>) {
        let x_end = (self.offset_x + viewport_width).min(self.content_width);
        let y_end = (self.offset_y + viewport_height).min(self.content_height);

        (self.offset_x..x_end, self.offset_y..y_end)
    }

    /// Get visible horizontal range only
    pub fn visible_x_range(&self, viewport_width: usize) -> Range<usize> {
        let end = (self.offset_x + viewport_width).min(self.content_width);
        self.offset_x..end
    }

    /// Get visible vertical range only
    pub fn visible_y_range(&self, viewport_height: usize) -> Range<usize> {
        let end = (self.offset_y + viewport_height).min(self.content_height);
        self.offset_y..end
    }

    /// Check if a position is currently visible
    pub fn is_visible(
        &self,
        x: usize,
        y: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) -> bool {
        x >= self.offset_x
            && x < self.offset_x + viewport_width
            && y >= self.offset_y
            && y < self.offset_y + viewport_height
    }

    /// Convert content coordinates to viewport coordinates
    ///
    /// Returns None if the position is outside the viewport.
    pub fn content_to_viewport(
        &self,
        x: usize,
        y: usize,
        viewport_width: usize,
        viewport_height: usize,
    ) -> Option<(usize, usize)> {
        if !self.is_visible(x, y, viewport_width, viewport_height) {
            return None;
        }

        Some((
            x.saturating_sub(self.offset_x),
            y.saturating_sub(self.offset_y),
        ))
    }

    /// Convert viewport coordinates to content coordinates
    pub fn viewport_to_content(&self, vx: usize, vy: usize) -> (usize, usize) {
        (self.offset_x + vx, self.offset_y + vy)
    }

    /// Calculate scrollbar position and size
    ///
    /// Returns (position, size) as ratios 0.0-1.0
    pub fn scrollbar_vertical(&self, viewport_height: usize) -> (f32, f32) {
        if self.content_height <= viewport_height {
            return (0.0, 1.0);
        }

        let ratio = viewport_height as f32 / self.content_height as f32;
        let pos = self.offset_y as f32 / (self.content_height - viewport_height) as f32;
        (pos.clamp(0.0, 1.0), ratio.clamp(0.0, 1.0))
    }

    /// Calculate horizontal scrollbar position and size
    pub fn scrollbar_horizontal(&self, viewport_width: usize) -> (f32, f32) {
        if self.content_width <= viewport_width {
            return (0.0, 1.0);
        }

        let ratio = viewport_width as f32 / self.content_width as f32;
        let pos = self.offset_x as f32 / (self.content_width - viewport_width) as f32;
        (pos.clamp(0.0, 1.0), ratio.clamp(0.0, 1.0))
    }

    /// Clamp offset to valid range
    fn clamp_offset(&mut self, viewport_width: usize, viewport_height: usize) {
        if self.content_width > viewport_width {
            self.offset_x = self
                .offset_x
                .min(self.content_width.saturating_sub(viewport_width));
        } else {
            self.offset_x = 0;
        }

        if self.content_height > viewport_height {
            self.offset_y = self
                .offset_y
                .min(self.content_height.saturating_sub(viewport_height));
        } else {
            self.offset_y = 0;
        }
    }
}

impl Default for ScrollableView {
    fn default() -> Self {
        ScrollableView::new(0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollable_creation() {
        let scroll = ScrollableView::new(1000, 500);
        assert_eq!(scroll.content_size(), (1000, 500));
        assert_eq!(scroll.offset(), (0, 0));
    }

    #[test]
    fn test_scroll_to() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.scroll_to(100, 50);
        assert_eq!(scroll.offset(), (100, 50));

        // Should clamp to content bounds
        scroll.scroll_to(2000, 1000);
        assert_eq!(scroll.offset(), (1000, 500));
    }

    #[test]
    fn test_scroll_by() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.scroll_to(100, 100);

        scroll.scroll_by(10, -20);
        assert_eq!(scroll.offset(), (110, 80));

        // Should not go negative
        scroll.scroll_by(-200, -200);
        assert_eq!(scroll.offset(), (0, 0));
    }

    #[test]
    fn test_visible_range() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.scroll_to(100, 50);

        let (x_range, y_range) = scroll.visible_range(80, 24);
        assert_eq!(x_range, 100..180);
        assert_eq!(y_range, 50..74);
    }

    #[test]
    fn test_visible_range_at_edge() {
        let mut scroll = ScrollableView::new(100, 50);
        scroll.scroll_to(50, 30);

        let (x_range, y_range) = scroll.visible_range(80, 24);
        // Should clamp to content size
        assert_eq!(x_range, 50..100);
        assert_eq!(y_range, 30..50);
    }

    #[test]
    fn test_ensure_visible() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.scroll_margin = 0; // No margin for simpler test

        // Position already visible - no scroll
        scroll.ensure_visible(10, 10, 80, 24);
        assert_eq!(scroll.offset(), (0, 0));

        // Position below viewport - scroll down
        scroll.ensure_visible(10, 50, 80, 24);
        assert!(scroll.offset_y() > 0);
        assert!(scroll.is_visible(10, 50, 80, 24));

        // Position to the right - scroll right
        scroll.scroll_to(0, 0);
        scroll.ensure_visible(100, 10, 80, 24);
        assert!(scroll.offset_x() > 0);
        assert!(scroll.is_visible(100, 10, 80, 24));
    }

    #[test]
    fn test_coordinate_conversion() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.scroll_to(100, 50);

        // Visible position
        let viewport = scroll.content_to_viewport(110, 60, 80, 24);
        assert_eq!(viewport, Some((10, 10)));

        // Invisible position
        let viewport = scroll.content_to_viewport(50, 30, 80, 24);
        assert_eq!(viewport, None);

        // Reverse conversion
        let content = scroll.viewport_to_content(10, 10);
        assert_eq!(content, (110, 60));
    }

    #[test]
    fn test_page_navigation() {
        let mut scroll = ScrollableView::new(100, 200);

        scroll.page_down(24);
        assert_eq!(scroll.offset_y(), 23); // 24 - 1 for overlap

        scroll.page_down(24);
        assert_eq!(scroll.offset_y(), 46);

        scroll.page_up(24);
        assert_eq!(scroll.offset_y(), 23);

        scroll.scroll_to_bottom(24);
        assert_eq!(scroll.offset_y(), 176); // 200 - 24
    }

    #[test]
    fn test_scrollbar_calculation() {
        let scroll = ScrollableView::new(100, 200);

        let (pos, size) = scroll.scrollbar_vertical(50);
        assert_eq!(pos, 0.0);
        assert!((size - 0.25).abs() < 0.01); // 50/200 = 0.25
    }

    #[test]
    fn test_center_on() {
        let mut scroll = ScrollableView::new(1000, 500);
        scroll.center_on(500, 250, 80, 24);

        // Should be roughly centered
        assert_eq!(scroll.offset_x(), 460); // 500 - 40
        assert_eq!(scroll.offset_y(), 238); // 250 - 12
    }
}

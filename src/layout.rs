//! Layout system - flex-based positioning and sizing

/// Rectangle bounds in character cells
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Create rectangle from terminal dimensions (fills entire screen)
    pub fn fullscreen(cols: u16, rows: u16) -> Self {
        Rect::new(0, 0, cols, rows)
    }

    /// Get right edge x-coordinate
    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    /// Get bottom edge y-coordinate
    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    /// Check if point is inside rectangle
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Create a subrect with padding applied
    pub fn inner(&self, padding: u16) -> Self {
        let padding2 = padding.saturating_mul(2);
        Rect {
            x: self.x.saturating_add(padding),
            y: self.y.saturating_add(padding),
            width: self.width.saturating_sub(padding2),
            height: self.height.saturating_sub(padding2),
        }
    }

    /// Split horizontally into top and bottom
    pub fn split_horizontal(&self, top_height: u16) -> (Rect, Rect) {
        let top = Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: top_height.min(self.height),
        };

        let bottom = Rect {
            x: self.x,
            y: self.y.saturating_add(top_height),
            width: self.width,
            height: self.height.saturating_sub(top_height),
        };

        (top, bottom)
    }

    /// Split vertically into left and right
    pub fn split_vertical(&self, left_width: u16) -> (Rect, Rect) {
        let left = Rect {
            x: self.x,
            y: self.y,
            width: left_width.min(self.width),
            height: self.height,
        };

        let right = Rect {
            x: self.x.saturating_add(left_width),
            y: self.y,
            width: self.width.saturating_sub(left_width),
            height: self.height,
        };

        (left, right)
    }
}

/// Flex direction for container layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

/// Alignment options for flex containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Start,
    Center,
    End,
    Stretch,
}

/// Size constraint for flex children
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// Fixed size in cells
    Fixed(u16),
    /// Proportional size (flex grow factor)
    Flex(u16),
    /// Size based on content (not yet implemented, acts as Flex(1))
    Auto,
}

/// Flex container layout calculator
#[derive(Debug, Clone)]
pub struct FlexLayout {
    direction: FlexDirection,
    gap: u16,
    padding: u16,
    align: Alignment,
}

impl FlexLayout {
    /// Create a new flex layout
    pub fn new(direction: FlexDirection) -> Self {
        FlexLayout {
            direction,
            gap: 0,
            padding: 0,
            align: Alignment::Stretch,
        }
    }

    /// Set gap between children
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Set padding around container
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    /// Set cross-axis alignment
    pub fn align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    /// Calculate child rectangles for given container and sizes
    pub fn layout(&self, container: Rect, sizes: &[Size]) -> Vec<Rect> {
        if sizes.is_empty() {
            return Vec::new();
        }

        let inner = container.inner(self.padding);
        let flex_unit_size = self.flex_unit_size(&inner, sizes);

        let mut rects = Vec::with_capacity(sizes.len());
        let mut offset = 0u16;

        for size in sizes {
            let child_main_size = match size {
                Size::Fixed(s) => *s,
                Size::Flex(f) => flex_unit_size.saturating_mul(*f),
                Size::Auto => flex_unit_size,
            };

            let rect = self.child_rect(&inner, offset, child_main_size);
            rects.push(rect);
            offset = offset
                .saturating_add(child_main_size)
                .saturating_add(self.gap);
        }

        rects
    }

    fn flex_unit_size(&self, inner: &Rect, sizes: &[Size]) -> u16 {
        let main_size = match self.direction {
            FlexDirection::Row => inner.width,
            FlexDirection::Column => inner.height,
        };

        let total_gap = self
            .gap
            .saturating_mul(sizes.len().saturating_sub(1) as u16);
        let available = main_size.saturating_sub(total_gap);

        let mut fixed_space = 0u16;
        let mut flex_units = 0u16;

        for size in sizes {
            match size {
                Size::Fixed(s) => fixed_space = fixed_space.saturating_add(*s),
                Size::Flex(f) => flex_units = flex_units.saturating_add(*f),
                Size::Auto => flex_units = flex_units.saturating_add(1),
            }
        }

        let flex_space = available.saturating_sub(fixed_space);
        if flex_units > 0 {
            flex_space / flex_units
        } else {
            0
        }
    }

    fn child_rect(&self, inner: &Rect, offset: u16, child_main_size: u16) -> Rect {
        match self.direction {
            FlexDirection::Row => {
                let x = inner.x.saturating_add(offset);
                let y = self.calculate_cross_offset(inner.y, inner.height, inner.height);
                Rect::new(x, y, child_main_size, inner.height)
            }
            FlexDirection::Column => {
                let x = self.calculate_cross_offset(inner.x, inner.width, inner.width);
                let y = inner.y.saturating_add(offset);
                Rect::new(x, y, inner.width, child_main_size)
            }
        }
    }

    /// Calculate offset for cross-axis alignment
    fn calculate_cross_offset(&self, base: u16, container_size: u16, child_size: u16) -> u16 {
        match self.align {
            Alignment::Start => base,
            Alignment::Center => {
                base.saturating_add((container_size.saturating_sub(child_size)) / 2)
            }
            Alignment::End => base.saturating_add(container_size.saturating_sub(child_size)),
            Alignment::Stretch => base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_creation() {
        let r = Rect::new(10, 20, 30, 40);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 30);
        assert_eq!(r.height, 40);
        assert_eq!(r.right(), 40);
        assert_eq!(r.bottom(), 60);
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(10, 10, 20, 20);
        assert!(r.contains(15, 15));
        assert!(r.contains(10, 10)); // edge
        assert!(!r.contains(30, 30)); // outside
        assert!(!r.contains(5, 15)); // left of rect
    }

    #[test]
    fn test_rect_split_horizontal() {
        let r = Rect::new(0, 0, 80, 24);
        let (top, bottom) = r.split_horizontal(3);

        assert_eq!(top, Rect::new(0, 0, 80, 3));
        assert_eq!(bottom, Rect::new(0, 3, 80, 21));
    }

    #[test]
    fn test_rect_split_vertical() {
        let r = Rect::new(0, 0, 80, 24);
        let (left, right) = r.split_vertical(20);

        assert_eq!(left, Rect::new(0, 0, 20, 24));
        assert_eq!(right, Rect::new(20, 0, 60, 24));
    }

    #[test]
    fn test_flex_layout_row() {
        let container = Rect::new(0, 0, 100, 10);
        let layout = FlexLayout::new(FlexDirection::Row);

        let sizes = vec![Size::Fixed(20), Size::Flex(1), Size::Fixed(20)];
        let rects = layout.layout(container, &sizes);

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].width, 20);
        assert_eq!(rects[1].width, 60); // 100 - 20 - 20
        assert_eq!(rects[2].width, 20);
    }

    #[test]
    fn test_flex_layout_column_with_gap() {
        let container = Rect::new(0, 0, 80, 24);
        let layout = FlexLayout::new(FlexDirection::Column).gap(1);

        let sizes = vec![Size::Fixed(3), Size::Flex(1), Size::Fixed(1)];
        let rects = layout.layout(container, &sizes);

        // Total: 3 + gap(1) + flex + gap(1) + 1 = 24
        // Flex gets: 24 - 3 - 1 - 2 = 18
        assert_eq!(rects[0].height, 3);
        assert_eq!(rects[1].height, 18);
        assert_eq!(rects[2].height, 1);

        // Check positions with gaps
        assert_eq!(rects[0].y, 0);
        assert_eq!(rects[1].y, 4); // 3 + 1 gap
        assert_eq!(rects[2].y, 23); // 4 + 18 + 1 gap
    }
}

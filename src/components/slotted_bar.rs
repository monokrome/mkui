//! Slotted bar component - flexible slot-based layout for headers and status bars

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::EventHandler;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Size specification for slot content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotSize {
    /// Fixed size in character blocks (terminal cells)
    Blocks(u16),
    /// Percentage of available width (0-100)
    Percent(u8),
    /// Fill remaining space (shared with other FILL slots)
    Fill,
}

/// Metadata for content that can be placed in a slot
/// Components that implement this trait provide sizing hints and priorities
/// Rendering is done through the Component trait
pub trait SlotContent: Component {
    /// List of sizes this component can render at, from largest to smallest
    /// The allocator tries each size in order until one fits
    /// If empty or None, component must be hidden when space is tight
    ///
    /// # Examples
    /// ```ignore
    /// // Title that can shrink gracefully
    /// vec![SlotSize::Fill, SlotSize::Percent(50), SlotSize::Blocks(30), SlotSize::Blocks(10)]
    ///
    /// // Fixed-size badge
    /// vec![SlotSize::Blocks(8)]
    ///
    /// // Flexible spacer
    /// vec![SlotSize::Fill]
    /// ```
    fn responsive_sizes(&self) -> Vec<SlotSize> {
        vec![SlotSize::Fill] // Default: flexible
    }

    /// Whether this slot can be hidden when space is tight
    /// Default: true (can be hidden based on priority)
    fn can_hide(&self) -> bool {
        true
    }

    /// Get as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// A slot in a slotted bar
pub struct Slot {
    content: Box<dyn SlotContent>,
    priority: u16,
}

impl Slot {
    /// Create a new slot with content and priority
    pub fn new(content: Box<dyn SlotContent>, priority: u16) -> Self {
        Slot { content, priority }
    }

    /// Create a high-priority slot
    pub fn high(content: Box<dyn SlotContent>) -> Self {
        Self::new(content, 100)
    }

    /// Create a medium-priority slot
    pub fn medium(content: Box<dyn SlotContent>) -> Self {
        Self::new(content, 50)
    }

    /// Create a low-priority slot
    pub fn low(content: Box<dyn SlotContent>) -> Self {
        Self::new(content, 10)
    }
}

/// Slotted bar component for headers and status bars
pub struct SlottedBar {
    slots: Vec<Slot>,
    background_style: String,
    dirty: bool,
}

impl SlottedBar {
    /// Create a new slotted bar
    pub fn new() -> Self {
        SlottedBar {
            slots: Vec::new(),
            background_style: "\x1b[7m".to_string(), // Default: inverse video
            dirty: true,
        }
    }

    /// Set the background style
    pub fn with_background(mut self, style: String) -> Self {
        self.background_style = style;
        self.dirty = true;
        self
    }

    /// Add a slot
    pub fn add_slot(&mut self, slot: Slot) {
        self.slots.push(slot);
        self.dirty = true;
    }

    /// Add content with priority
    pub fn add(&mut self, content: Box<dyn SlotContent>, priority: u16) {
        self.add_slot(Slot::new(content, priority));
    }

    /// Get mutable reference to a slot's content by index
    pub fn get_slot_mut(&mut self, idx: usize) -> Option<&mut Box<dyn SlotContent>> {
        self.slots.get_mut(idx).map(|s| &mut s.content)
    }

    /// Calculate slot widths based on available space and priorities
    /// Hides low-priority slots when space is tight
    /// Calculate widths for all slots given available width
    /// Returns vector of (slot_index, allocated_width) tuples
    fn calculate_widths(&self, available_width: u16) -> Vec<(usize, u16)> {
        if self.slots.is_empty() {
            return Vec::new();
        }

        // Build slot info with responsive sizes
        let mut slot_info: Vec<(usize, u16, Vec<SlotSize>, bool)> = self
            .slots
            .iter()
            .enumerate()
            .map(|(idx, slot)| {
                let sizes = slot.content.responsive_sizes();
                let can_hide = slot.content.can_hide();
                (idx, slot.priority, sizes, can_hide)
            })
            .collect();

        // Sort by priority (highest first)
        slot_info.sort_by(|a, b| b.1.cmp(&a.1));

        // Try to allocate, hiding slots if needed
        let mut visible_slots = slot_info.clone();
        loop {
            if let Some(allocations) = self.try_allocate(&visible_slots, available_width) {
                return allocations;
            }

            // Couldn't fit - remove lowest priority hideable slot
            if let Some(pos) = visible_slots
                .iter()
                .rposition(|(_, _, _, can_hide)| *can_hide)
            {
                visible_slots.remove(pos);
            } else {
                // No more hideable slots - allocate what we can
                return visible_slots
                    .iter()
                    .map(|(idx, _, _, _)| (*idx, 0))
                    .collect();
            }
        }
    }

    /// Try to allocate space, returns Some(allocations) if successful, None if doesn't fit
    fn try_allocate(
        &self,
        slot_info: &[(usize, u16, Vec<SlotSize>, bool)],
        available_width: u16,
    ) -> Option<Vec<(usize, u16)>> {
        // Try to find a combination of sizes that fits
        // Start with the largest size for each slot and work down

        let num_slots = slot_info.len();
        let mut size_indices = vec![0usize; num_slots]; // Index into each slot's responsive_sizes

        loop {
            // Calculate widths for current size combination
            if let Some(allocations) = self.resolve_sizes(slot_info, &size_indices, available_width)
            {
                return Some(allocations);
            }

            // Try next combination (increment rightmost index that can increment)
            let mut incremented = false;
            for i in (0..num_slots).rev() {
                if size_indices[i] + 1 < slot_info[i].2.len() {
                    size_indices[i] += 1;
                    // Reset all indices to the right
                    for idx in size_indices.iter_mut().take(num_slots).skip(i + 1) {
                        *idx = 0;
                    }
                    incremented = true;
                    break;
                }
            }

            if !incremented {
                // Tried all combinations, none fit
                return None;
            }
        }
    }

    /// Resolve SlotSizes to actual widths, returns Some if fits, None if doesn't fit
    fn resolve_sizes(
        &self,
        slot_info: &[(usize, u16, Vec<SlotSize>, bool)],
        size_indices: &[usize],
        available_width: u16,
    ) -> Option<Vec<(usize, u16)>> {
        // First pass: calculate fixed sizes (Blocks and Percent)
        let mut allocations: Vec<(usize, Option<u16>)> = Vec::new();
        let mut fill_indices = Vec::new();
        let mut used_width = 0u16;

        for (i, (idx, _, sizes, _)) in slot_info.iter().enumerate() {
            let size = &sizes[size_indices[i]];

            match size {
                SlotSize::Blocks(blocks) => {
                    allocations.push((*idx, Some(*blocks)));
                    used_width = used_width.saturating_add(*blocks);
                }
                SlotSize::Percent(pct) => {
                    let width = ((available_width as u32 * (*pct as u32)) / 100) as u16;
                    allocations.push((*idx, Some(width)));
                    used_width = used_width.saturating_add(width);
                }
                SlotSize::Fill => {
                    allocations.push((*idx, None)); // Resolve later
                    fill_indices.push(i);
                }
            }
        }

        // Check if fixed sizes already overflow
        if used_width > available_width {
            return None;
        }

        // Second pass: distribute remaining space to FILL slots
        let remaining = available_width.saturating_sub(used_width);

        if !fill_indices.is_empty() {
            let per_fill = remaining / fill_indices.len() as u16;
            let leftover = remaining % fill_indices.len() as u16;

            for (i, &fill_idx) in fill_indices.iter().enumerate() {
                let extra = if i == 0 { leftover } else { 0 };
                let fill_width = per_fill + extra;
                allocations[fill_idx].1 = Some(fill_width);
            }
        }

        // Convert to final format: all should have widths now
        let final_allocations: Vec<(usize, u16)> = allocations
            .into_iter()
            .map(|(idx, width)| (idx, width.unwrap_or(0)))
            .collect();

        // Sort by original index
        let mut sorted = final_allocations;
        sorted.sort_by_key(|(idx, _)| *idx);

        Some(sorted)
    }
}

impl Default for SlottedBar {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for SlottedBar {
    fn handle_event(&mut self, event: &crate::event::Event) -> bool {
        for slot in &mut self.slots {
            if slot.content.handle_event(event) {
                return true;
            }
        }
        false
    }
}

impl Component for SlottedBar {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        // Clear the bar with background style (if any)
        if !self.background_style.is_empty() {
            renderer.move_cursor(bounds.x, bounds.y)?;
            renderer.write_styled(&" ".repeat(bounds.width as usize), &self.background_style)?;
        }

        // Calculate slot widths
        let widths = self.calculate_widths(bounds.width);

        // Render each slot - components must respect their allocated bounds
        let mut x_offset = bounds.x;
        for (idx, allocated_width) in widths {
            if allocated_width > 0 {
                let slot_bounds = Rect::new(x_offset, bounds.y, allocated_width, bounds.height);

                // Components receive their exact allocated width via bounds
                // They must render within these bounds (no overflow)
                self.slots[idx].content.render(renderer, slot_bounds, ctx)?;

                x_offset = x_offset.saturating_add(allocated_width);
            }
        }

        self.dirty = false;
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "SlottedBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSlotContent {
        width: Option<u16>,
    }

    impl EventHandler for TestSlotContent {}

    impl Component for TestSlotContent {
        fn render(
            &mut self,
            _renderer: &mut Renderer,
            _bounds: Rect,
            _ctx: &RenderContext,
        ) -> Result<()> {
            Ok(())
        }

        fn min_size(&self) -> (u16, u16) {
            (0, 1)
        }

        fn mark_dirty(&mut self) {}

        fn is_dirty(&self) -> bool {
            false
        }

        fn name(&self) -> &str {
            "TestSlot"
        }
    }

    impl SlotContent for TestSlotContent {
        fn responsive_sizes(&self) -> Vec<SlotSize> {
            if let Some(w) = self.width {
                vec![SlotSize::Blocks(w)]
            } else {
                vec![SlotSize::Fill]
            }
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_width_calculation() {
        let mut bar = SlottedBar::new();

        // Add slots with different preferred widths
        bar.add(Box::new(TestSlotContent { width: Some(10) }), 100); // High priority
        bar.add(Box::new(TestSlotContent { width: Some(20) }), 50); // Medium priority
        bar.add(Box::new(TestSlotContent { width: None }), 10); // Low priority, flexible

        let widths = bar.calculate_widths(80);

        // Should allocate by priority
        assert_eq!(widths.len(), 3);
        assert_eq!(widths[0].1, 10); // High priority gets its preferred
        assert_eq!(widths[1].1, 20); // Medium priority gets its preferred
        assert_eq!(widths[2].1, 50); // Flexible gets remainder (80 - 10 - 20)
    }
}

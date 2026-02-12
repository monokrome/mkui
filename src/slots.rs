//! Slot system - React-like context for registering content to named slots
//!
//! Provides separate slot containers for different UI regions (header, status bar).
//! Works like Theme - passed through RenderContext during rendering.
//! Application owns the slot containers, updates them, and passes references to RenderContext.
//!
//! ## Priority System
//!
//! Slots support priority-based layering where higher-priority content overrides
//! lower-priority content. When the higher-priority content is cleared, the lower-priority
//! content is automatically restored.
//!
//! Priority levels (highest to lowest):
//! - `OVERLAY` (100): Modal overlays, popups
//! - `TEMPORARY` (75): Temporary status messages, notifications
//! - `PLUGIN` (50): Plugin UI content
//! - `NORMAL` (25): Standard application content (default)
//! - `DEFAULT` (0): Fallback/placeholder content

use std::collections::HashMap;

/// Priority levels for slot content
pub mod priority {
    /// Overlay priority - for modal dialogs, popups (highest)
    pub const OVERLAY: u8 = 100;
    /// Temporary priority - for status messages, notifications
    pub const TEMPORARY: u8 = 75;
    /// Plugin priority - for plugin UI content
    pub const PLUGIN: u8 = 50;
    /// Normal priority - for standard application content (default)
    pub const NORMAL: u8 = 25;
    /// Default priority - for fallback/placeholder content (lowest)
    pub const DEFAULT: u8 = 0;
}

/// An entry in a slot's priority stack
#[derive(Clone, Debug, PartialEq)]
struct SlotEntry {
    content: SlotContent,
    priority: u8,
}

/// A priority-aware slot that stacks content at different priorities
#[derive(Clone, Debug, Default)]
struct PrioritySlot {
    /// Stack of entries sorted by priority (highest first)
    entries: Vec<SlotEntry>,
}

impl PrioritySlot {
    /// Set content at a given priority, replacing any existing content at that priority
    fn set(&mut self, content: SlotContent, priority: u8) {
        // Remove existing entry at this priority
        self.entries.retain(|e| e.priority != priority);
        // Insert new entry
        self.entries.push(SlotEntry { content, priority });
        // Sort by priority descending (highest first)
        self.entries.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Clear content at a given priority
    fn clear(&mut self, priority: u8) {
        self.entries.retain(|e| e.priority != priority);
    }

    /// Clear all content at all priorities
    #[allow(dead_code)]
    pub(crate) fn clear_all(&mut self) {
        self.entries.clear();
    }

    /// Get the highest-priority content
    fn get(&self) -> Option<&SlotContent> {
        self.entries.first().map(|e| &e.content)
    }

    /// Get content at a specific priority level
    fn get_at_priority(&self, priority: u8) -> Option<&SlotContent> {
        self.entries
            .iter()
            .find(|e| e.priority == priority)
            .map(|e| &e.content)
    }

    /// Check if there's any content
    fn is_empty(&self) -> bool {
        self.entries.is_empty() || self.entries.iter().all(|e| e.content.is_empty())
    }

    /// Get the priority of the current (highest) content
    fn current_priority(&self) -> Option<u8> {
        self.entries.first().map(|e| e.priority)
    }
}

/// Content that can be placed in a slot
#[derive(Clone, Debug, PartialEq)]
pub enum SlotContent {
    /// Simple text content
    Text(String),
    /// Text with style class for themed rendering
    Styled { text: String, class: String },
}

impl SlotContent {
    pub fn text(s: impl Into<String>) -> Self {
        SlotContent::Text(s.into())
    }

    pub fn styled(text: impl Into<String>, class: impl Into<String>) -> Self {
        SlotContent::Styled {
            text: text.into(),
            class: class.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            SlotContent::Text(s) => s,
            SlotContent::Styled { text, .. } => text,
        }
    }

    pub fn style_class(&self) -> Option<&str> {
        match self {
            SlotContent::Styled { class, .. } => Some(class),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl Default for SlotContent {
    fn default() -> Self {
        SlotContent::Text(String::new())
    }
}

impl From<String> for SlotContent {
    fn from(s: String) -> Self {
        SlotContent::Text(s)
    }
}

impl From<&str> for SlotContent {
    fn from(s: &str) -> Self {
        SlotContent::Text(s.to_string())
    }
}

/// Well-known slot names for header
pub mod header_slots {
    pub const LEFT: &str = "left";
    pub const CENTER: &str = "center";
    pub const RIGHT: &str = "right";
    pub const TITLE: &str = "title";
}

/// Well-known slot names for status bar
pub mod status_slots {
    pub const LEFT: &str = "left";
    pub const CENTER: &str = "center";
    pub const RIGHT: &str = "right";
    pub const MODE: &str = "mode";
    pub const MESSAGE: &str = "message";
    pub const POSITION: &str = "position";
    pub const COMMAND: &str = "command"; // For command line input
}

/// A container for slots in a specific UI region with priority support
#[derive(Clone, Debug, Default)]
pub struct RegionSlots {
    slots: HashMap<String, PrioritySlot>,
}

impl RegionSlots {
    /// Create a new empty slot container
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    /// Set content for a slot at NORMAL priority (backward compatible)
    pub fn set(&mut self, slot: &str, content: impl Into<SlotContent>) {
        self.set_at_priority(slot, content, priority::NORMAL);
    }

    /// Set content for a slot at a specific priority
    pub fn set_at_priority(&mut self, slot: &str, content: impl Into<SlotContent>, prio: u8) {
        self.slots
            .entry(slot.to_string())
            .or_default()
            .set(content.into(), prio);
    }

    /// Set content only if it differs from current value at NORMAL priority
    /// Returns true if the value was changed
    pub fn set_if_changed(&mut self, slot: &str, content: &str) -> bool {
        self.set_if_changed_at_priority(slot, content, priority::NORMAL)
    }

    /// Set content only if it differs from current value at a specific priority
    /// Returns true if the value was changed
    pub fn set_if_changed_at_priority(&mut self, slot: &str, content: &str, prio: u8) -> bool {
        if let Some(ps) = self.slots.get(slot) {
            if let Some(existing) = ps.get_at_priority(prio) {
                if existing.as_str() == content {
                    return false; // No change, no allocation
                }
            }
        }
        self.set_at_priority(slot, SlotContent::Text(content.to_string()), prio);
        true
    }

    /// Set styled content only if it differs from current value
    /// Returns true if the value was changed
    pub fn set_styled_if_changed(&mut self, slot: &str, text: &str, class: &str) -> bool {
        self.set_styled_if_changed_at_priority(slot, text, class, priority::NORMAL)
    }

    /// Set styled content only if it differs from current value at a specific priority
    /// Returns true if the value was changed
    pub fn set_styled_if_changed_at_priority(
        &mut self,
        slot: &str,
        text: &str,
        class: &str,
        prio: u8,
    ) -> bool {
        if let Some(ps) = self.slots.get(slot) {
            if let Some(existing) = ps.get_at_priority(prio) {
                if existing.as_str() == text && existing.style_class() == Some(class) {
                    return false; // No change, no allocation
                }
            }
        }
        self.set_at_priority(
            slot,
            SlotContent::Styled {
                text: text.to_string(),
                class: class.to_string(),
            },
            prio,
        );
        true
    }

    /// Clear a slot at NORMAL priority (backward compatible)
    pub fn clear(&mut self, slot: &str) {
        self.clear_at_priority(slot, priority::NORMAL);
    }

    /// Clear a slot at a specific priority
    pub fn clear_at_priority(&mut self, slot: &str, prio: u8) {
        if let Some(ps) = self.slots.get_mut(slot) {
            ps.clear(prio);
            // Clean up empty slots
            if ps.is_empty() {
                self.slots.remove(slot);
            }
        }
    }

    /// Clear all content at all priorities for a slot
    pub fn clear_all(&mut self, slot: &str) {
        self.slots.remove(slot);
    }

    /// Clear a slot only if it currently has content at NORMAL priority
    /// Returns true if the slot was cleared
    pub fn clear_if_set(&mut self, slot: &str) -> bool {
        self.clear_if_set_at_priority(slot, priority::NORMAL)
    }

    /// Clear a slot only if it currently has content at a specific priority
    /// Returns true if the slot was cleared
    pub fn clear_if_set_at_priority(&mut self, slot: &str, prio: u8) -> bool {
        if let Some(ps) = self.slots.get_mut(slot) {
            if ps.get_at_priority(prio).is_some() {
                ps.clear(prio);
                if ps.is_empty() {
                    self.slots.remove(slot);
                }
                return true;
            }
        }
        false
    }

    /// Get the highest-priority content for a slot
    pub fn get(&self, slot: &str) -> Option<&SlotContent> {
        self.slots.get(slot).and_then(|ps| ps.get())
    }

    /// Get text content for a slot (highest priority)
    pub fn get_text(&self, slot: &str) -> &str {
        self.slots
            .get(slot)
            .and_then(|ps| ps.get())
            .map(|c| c.as_str())
            .unwrap_or("")
    }

    /// Check if a slot has non-empty content (at any priority)
    pub fn has(&self, slot: &str) -> bool {
        self.slots
            .get(slot)
            .map(|ps| !ps.is_empty())
            .unwrap_or(false)
    }

    /// Get the current priority level of a slot's visible content
    pub fn current_priority(&self, slot: &str) -> Option<u8> {
        self.slots.get(slot).and_then(|ps| ps.current_priority())
    }

    /// Get all slot names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.slots.keys().map(|s| s.as_str())
    }
}

/// All slot containers - owned by Application, passed to RenderContext
#[derive(Clone, Debug, Default)]
pub struct Slots {
    /// Header bar slots
    pub header: RegionSlots,
    /// Status bar slots
    pub status: RegionSlots,
}

impl Slots {
    /// Create new empty slot containers
    pub fn new() -> Self {
        Self {
            header: RegionSlots::new(),
            status: RegionSlots::new(),
        }
    }
}

/// Hook trait for accessing slots from context (like UseTheme)
pub trait UseSlots {
    /// Get header slot content
    fn use_header_slot<'a>(&self, ctx: &'a crate::context::RenderContext, slot: &str) -> &'a str {
        ctx.slots.header.get_text(slot)
    }

    /// Get status bar slot content
    fn use_status_slot<'a>(&self, ctx: &'a crate::context::RenderContext, slot: &str) -> &'a str {
        ctx.slots.status.get_text(slot)
    }

    /// Check if header slot has content
    fn use_header_has(&self, ctx: &crate::context::RenderContext, slot: &str) -> bool {
        ctx.slots.header.has(slot)
    }

    /// Check if status slot has content
    fn use_status_has(&self, ctx: &crate::context::RenderContext, slot: &str) -> bool {
        ctx.slots.status.has(slot)
    }
}

/// Auto-implement for all types
impl<T> UseSlots for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_slots() {
        let mut region = RegionSlots::new();

        // Set content
        region.set(status_slots::LEFT, "NORMAL");
        assert_eq!(region.get_text(status_slots::LEFT), "NORMAL");

        // Overwrite
        region.set(status_slots::LEFT, "OVERRIDE");
        assert_eq!(region.get_text(status_slots::LEFT), "OVERRIDE");

        // Check has
        assert!(region.has(status_slots::LEFT));
        assert!(!region.has(status_slots::RIGHT));
    }

    #[test]
    fn test_slot_clear() {
        let mut region = RegionSlots::new();
        region.set(status_slots::CENTER, "message");
        assert!(region.has(status_slots::CENTER));

        region.clear(status_slots::CENTER);
        assert!(!region.has(status_slots::CENTER));
    }

    #[test]
    fn test_slots_container() {
        let mut slots = Slots::new();

        // Header and status are separate
        slots.header.set(header_slots::TITLE, "My App");
        slots.status.set(status_slots::MODE, "NORMAL");

        assert_eq!(slots.header.get_text(header_slots::TITLE), "My App");
        assert_eq!(slots.status.get_text(status_slots::MODE), "NORMAL");

        // They don't interfere with each other
        assert!(!slots.header.has(status_slots::MODE));
        assert!(!slots.status.has(header_slots::TITLE));
    }

    #[test]
    fn test_styled_content() {
        let mut region = RegionSlots::new();
        region.set(
            status_slots::MODE,
            SlotContent::styled("INSERT", "mode_insert"),
        );

        let content = region.get(status_slots::MODE).unwrap();
        assert_eq!(content.as_str(), "INSERT");
        assert_eq!(content.style_class(), Some("mode_insert"));
    }

    #[test]
    fn test_set_if_changed() {
        let mut region = RegionSlots::new();

        // First set should change
        assert!(region.set_if_changed(status_slots::MODE, "NORMAL"));
        assert_eq!(region.get_text(status_slots::MODE), "NORMAL");

        // Same value should not change
        assert!(!region.set_if_changed(status_slots::MODE, "NORMAL"));

        // Different value should change
        assert!(region.set_if_changed(status_slots::MODE, "INSERT"));
        assert_eq!(region.get_text(status_slots::MODE), "INSERT");
    }

    #[test]
    fn test_set_styled_if_changed() {
        let mut region = RegionSlots::new();

        // First set should change
        assert!(region.set_styled_if_changed(status_slots::MODE, "NORMAL", "mode_normal"));

        // Same text and class should not change
        assert!(!region.set_styled_if_changed(status_slots::MODE, "NORMAL", "mode_normal"));

        // Different text should change
        assert!(region.set_styled_if_changed(status_slots::MODE, "INSERT", "mode_normal"));

        // Different class should change
        assert!(region.set_styled_if_changed(status_slots::MODE, "INSERT", "mode_insert"));
    }

    #[test]
    fn test_clear_if_set() {
        let mut region = RegionSlots::new();

        // Clear on empty should return false
        assert!(!region.clear_if_set(status_slots::MODE));

        // Set and clear should return true
        region.set(status_slots::MODE, "NORMAL");
        assert!(region.clear_if_set(status_slots::MODE));

        // Second clear should return false
        assert!(!region.clear_if_set(status_slots::MODE));
    }

    #[test]
    fn test_priority_slot_basic() {
        let mut ps = PrioritySlot::default();

        // Set at normal priority
        ps.set(SlotContent::text("normal"), priority::NORMAL);
        assert_eq!(ps.get().unwrap().as_str(), "normal");
        assert_eq!(ps.current_priority(), Some(priority::NORMAL));
    }

    #[test]
    fn test_priority_slot_layering() {
        let mut ps = PrioritySlot::default();

        // Set normal priority content
        ps.set(SlotContent::text("normal"), priority::NORMAL);
        assert_eq!(ps.get().unwrap().as_str(), "normal");

        // Higher priority overrides
        ps.set(SlotContent::text("temporary"), priority::TEMPORARY);
        assert_eq!(ps.get().unwrap().as_str(), "temporary");
        assert_eq!(ps.current_priority(), Some(priority::TEMPORARY));

        // Even higher priority overrides
        ps.set(SlotContent::text("overlay"), priority::OVERLAY);
        assert_eq!(ps.get().unwrap().as_str(), "overlay");

        // Clear overlay reveals temporary
        ps.clear(priority::OVERLAY);
        assert_eq!(ps.get().unwrap().as_str(), "temporary");

        // Clear temporary reveals normal
        ps.clear(priority::TEMPORARY);
        assert_eq!(ps.get().unwrap().as_str(), "normal");

        // Clear normal leaves empty
        ps.clear(priority::NORMAL);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_region_slots_priority() {
        let mut region = RegionSlots::new();

        // Set normal content
        region.set(status_slots::MESSAGE, "Status: OK");
        assert_eq!(region.get_text(status_slots::MESSAGE), "Status: OK");

        // Temporary message overrides
        region.set_at_priority(status_slots::MESSAGE, "File saved!", priority::TEMPORARY);
        assert_eq!(region.get_text(status_slots::MESSAGE), "File saved!");
        assert_eq!(
            region.current_priority(status_slots::MESSAGE),
            Some(priority::TEMPORARY)
        );

        // Clear temporary reveals normal
        region.clear_at_priority(status_slots::MESSAGE, priority::TEMPORARY);
        assert_eq!(region.get_text(status_slots::MESSAGE), "Status: OK");
        assert_eq!(
            region.current_priority(status_slots::MESSAGE),
            Some(priority::NORMAL)
        );
    }

    #[test]
    fn test_priority_set_if_changed() {
        let mut region = RegionSlots::new();

        // Set at temporary priority
        assert!(region.set_if_changed_at_priority(
            status_slots::MESSAGE,
            "temp msg",
            priority::TEMPORARY
        ));

        // Same value at same priority should not change
        assert!(!region.set_if_changed_at_priority(
            status_slots::MESSAGE,
            "temp msg",
            priority::TEMPORARY
        ));

        // Same value at different priority SHOULD change (it's a different layer)
        assert!(region.set_if_changed_at_priority(
            status_slots::MESSAGE,
            "temp msg",
            priority::NORMAL
        ));

        // Now temp is still visible (higher priority)
        assert_eq!(region.get_text(status_slots::MESSAGE), "temp msg");
        assert_eq!(
            region.current_priority(status_slots::MESSAGE),
            Some(priority::TEMPORARY)
        );
    }

    #[test]
    fn test_clear_all_priorities() {
        let mut region = RegionSlots::new();

        // Set at multiple priorities
        region.set_at_priority(status_slots::MESSAGE, "default", priority::DEFAULT);
        region.set_at_priority(status_slots::MESSAGE, "normal", priority::NORMAL);
        region.set_at_priority(status_slots::MESSAGE, "temp", priority::TEMPORARY);

        // Visible is temp
        assert_eq!(region.get_text(status_slots::MESSAGE), "temp");

        // Clear all removes everything
        region.clear_all(status_slots::MESSAGE);
        assert!(!region.has(status_slots::MESSAGE));
        assert_eq!(region.get_text(status_slots::MESSAGE), "");
    }
}

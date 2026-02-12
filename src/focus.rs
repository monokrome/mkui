//! Focus management system for component navigation
//!
//! Provides centralized focus tracking with Tab/Shift-Tab navigation support.
//!
//! # Example
//!
//! ```ignore
//! let mut focus = FocusManager::new();
//! focus.register("input1");
//! focus.register("input2");
//! focus.register("button");
//!
//! focus.focus("input1");
//! assert!(focus.is_focused("input1"));
//!
//! focus.focus_next(); // Moves to input2
//! focus.focus_prev(); // Back to input1
//! ```

use std::collections::HashMap;

/// Unique identifier for a focusable component
pub type ComponentId = String;

/// Focus ring navigation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    /// Move to next focusable component (Tab)
    Next,
    /// Move to previous focusable component (Shift+Tab)
    Previous,
    /// Move up (k or arrow up)
    Up,
    /// Move down (j or arrow down)
    Down,
    /// Move left (h or arrow left)
    Left,
    /// Move right (l or arrow right)
    Right,
}

/// Metadata about a focusable component
#[derive(Debug, Clone)]
pub struct FocusableInfo {
    /// Component identifier
    pub id: ComponentId,
    /// Whether this component can receive focus
    pub focusable: bool,
    /// Tab order index (lower = earlier in tab order)
    pub tab_index: i32,
    /// Group for spatial navigation (components in same group navigate together)
    pub group: Option<String>,
}

impl FocusableInfo {
    /// Create new focusable info with defaults
    pub fn new(id: impl Into<ComponentId>) -> Self {
        Self {
            id: id.into(),
            focusable: true,
            tab_index: 0,
            group: None,
        }
    }

    /// Set the tab index
    pub fn with_tab_index(mut self, index: i32) -> Self {
        self.tab_index = index;
        self
    }

    /// Set the focus group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Set whether this component can be focused
    pub fn with_focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }
}

/// Centralized focus management
///
/// Tracks which component has focus and provides navigation between
/// focusable components using Tab/Shift-Tab or directional keys.
#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    /// Currently focused component ID
    focused_id: Option<ComponentId>,

    /// Registered focusable components in order
    focus_order: Vec<FocusableInfo>,

    /// Quick lookup by ID
    id_to_index: HashMap<ComponentId, usize>,

    /// Whether to show visual focus indicators
    focus_ring_visible: bool,

    /// Whether focus wraps around at boundaries
    wrap_around: bool,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focused_id: None,
            focus_order: Vec::new(),
            id_to_index: HashMap::new(),
            focus_ring_visible: true,
            wrap_around: true,
        }
    }

    /// Register a component as focusable
    pub fn register(&mut self, id: impl Into<ComponentId>) {
        self.register_with_info(FocusableInfo::new(id));
    }

    /// Register a component with custom focus info
    pub fn register_with_info(&mut self, info: FocusableInfo) {
        if self.id_to_index.contains_key(&info.id) {
            // Already registered, update info
            if let Some(&idx) = self.id_to_index.get(&info.id) {
                self.focus_order[idx] = info;
            }
        } else {
            let idx = self.focus_order.len();
            self.id_to_index.insert(info.id.clone(), idx);
            self.focus_order.push(info);
        }
        self.sort_by_tab_index();
    }

    /// Unregister a component
    pub fn unregister(&mut self, id: &str) {
        if let Some(&idx) = self.id_to_index.get(id) {
            // Clear focus if this component was focused
            if self.focused_id.as_deref() == Some(id) {
                self.focused_id = None;
            }

            self.focus_order.remove(idx);
            self.id_to_index.remove(id);

            // Rebuild index map
            self.id_to_index.clear();
            for (i, info) in self.focus_order.iter().enumerate() {
                self.id_to_index.insert(info.id.clone(), i);
            }
        }
    }

    /// Sort focus order by tab index
    fn sort_by_tab_index(&mut self) {
        self.focus_order.sort_by_key(|info| info.tab_index);
        self.id_to_index.clear();
        for (i, info) in self.focus_order.iter().enumerate() {
            self.id_to_index.insert(info.id.clone(), i);
        }
    }

    /// Focus a specific component by ID
    pub fn focus(&mut self, id: impl Into<ComponentId>) -> bool {
        let id = id.into();
        if let Some(&idx) = self.id_to_index.get(&id) {
            if self.focus_order[idx].focusable {
                self.focused_id = Some(id);
                return true;
            }
        }
        false
    }

    /// Clear focus (no component focused)
    pub fn blur(&mut self) {
        self.focused_id = None;
    }

    /// Move focus to the next focusable component
    pub fn focus_next(&mut self) -> bool {
        self.move_focus(FocusDirection::Next)
    }

    /// Move focus to the previous focusable component
    pub fn focus_prev(&mut self) -> bool {
        self.move_focus(FocusDirection::Previous)
    }

    /// Move focus in a direction
    pub fn move_focus(&mut self, direction: FocusDirection) -> bool {
        let focusable: Vec<_> = self
            .focus_order
            .iter()
            .enumerate()
            .filter(|(_, info)| info.focusable)
            .collect();

        if focusable.is_empty() {
            return false;
        }

        let current_idx = self
            .focused_id
            .as_ref()
            .and_then(|id| self.id_to_index.get(id))
            .and_then(|&idx| focusable.iter().position(|(i, _)| *i == idx));

        let new_idx = self.next_focus_index(current_idx, focusable.len(), direction);

        if let Some(idx) = new_idx {
            let (_, info) = &focusable[idx];
            self.focused_id = Some(info.id.clone());
            true
        } else {
            false
        }
    }

    fn next_focus_index(
        &self,
        current: Option<usize>,
        len: usize,
        direction: FocusDirection,
    ) -> Option<usize> {
        let forward = matches!(
            direction,
            FocusDirection::Next | FocusDirection::Down | FocusDirection::Right
        );

        match current {
            None if forward => Some(0),
            None => Some(len - 1),
            Some(idx) if forward && idx + 1 < len => Some(idx + 1),
            Some(idx) if !forward && idx > 0 => Some(idx - 1),
            Some(_) if self.wrap_around && forward => Some(0),
            Some(_) if self.wrap_around => Some(len - 1),
            Some(_) => None,
        }
    }

    /// Check if a specific component has focus
    pub fn is_focused(&self, id: &str) -> bool {
        self.focused_id.as_deref() == Some(id)
    }

    /// Get the currently focused component ID
    pub fn focused(&self) -> Option<&str> {
        self.focused_id.as_deref()
    }

    /// Check if focus ring should be visible
    pub fn is_focus_ring_visible(&self) -> bool {
        self.focus_ring_visible
    }

    /// Set focus ring visibility
    pub fn set_focus_ring_visible(&mut self, visible: bool) {
        self.focus_ring_visible = visible;
    }

    /// Set whether focus wraps around at boundaries
    pub fn set_wrap_around(&mut self, wrap: bool) {
        self.wrap_around = wrap;
    }

    /// Get the number of registered focusable components
    pub fn count(&self) -> usize {
        self.focus_order
            .iter()
            .filter(|info| info.focusable)
            .count()
    }

    /// Check if a component is registered
    pub fn is_registered(&self, id: &str) -> bool {
        self.id_to_index.contains_key(id)
    }

    /// Get all registered component IDs in focus order
    pub fn focus_order(&self) -> impl Iterator<Item = &str> {
        self.focus_order
            .iter()
            .filter(|info| info.focusable)
            .map(|info| info.id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_focus() {
        let mut fm = FocusManager::new();
        fm.register("a");
        fm.register("b");
        fm.register("c");

        assert_eq!(fm.focused(), None);

        fm.focus("b");
        assert!(fm.is_focused("b"));
        assert!(!fm.is_focused("a"));
    }

    #[test]
    fn test_focus_next_prev() {
        let mut fm = FocusManager::new();
        fm.register("a");
        fm.register("b");
        fm.register("c");

        fm.focus("a");

        fm.focus_next();
        assert!(fm.is_focused("b"));

        fm.focus_next();
        assert!(fm.is_focused("c"));

        fm.focus_next(); // Wraps
        assert!(fm.is_focused("a"));

        fm.focus_prev(); // Wraps back
        assert!(fm.is_focused("c"));
    }

    #[test]
    fn test_tab_index_ordering() {
        let mut fm = FocusManager::new();
        fm.register_with_info(FocusableInfo::new("c").with_tab_index(3));
        fm.register_with_info(FocusableInfo::new("a").with_tab_index(1));
        fm.register_with_info(FocusableInfo::new("b").with_tab_index(2));

        fm.focus_next(); // Should focus 'a' (tab_index 1)
        assert!(fm.is_focused("a"));

        fm.focus_next(); // Should focus 'b' (tab_index 2)
        assert!(fm.is_focused("b"));

        fm.focus_next(); // Should focus 'c' (tab_index 3)
        assert!(fm.is_focused("c"));
    }

    #[test]
    fn test_unfocusable_components() {
        let mut fm = FocusManager::new();
        fm.register_with_info(FocusableInfo::new("a"));
        fm.register_with_info(FocusableInfo::new("b").with_focusable(false));
        fm.register_with_info(FocusableInfo::new("c"));

        fm.focus("a");
        fm.focus_next();
        // Should skip 'b' and go to 'c'
        assert!(fm.is_focused("c"));
    }

    #[test]
    fn test_unregister() {
        let mut fm = FocusManager::new();
        fm.register("a");
        fm.register("b");
        fm.register("c");

        fm.focus("b");
        assert!(fm.is_focused("b"));

        fm.unregister("b");
        assert_eq!(fm.focused(), None);
        assert!(!fm.is_registered("b"));
        assert_eq!(fm.count(), 2);
    }

    #[test]
    fn test_no_wrap() {
        let mut fm = FocusManager::new();
        fm.set_wrap_around(false);
        fm.register("a");
        fm.register("b");

        fm.focus("b");
        let moved = fm.focus_next();
        assert!(!moved); // Can't move past end
        assert!(fm.is_focused("b"));

        fm.focus("a");
        let moved = fm.focus_prev();
        assert!(!moved); // Can't move past start
        assert!(fm.is_focused("a"));
    }
}

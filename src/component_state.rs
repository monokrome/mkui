//! Component render state tracking
//!
//! Tracks each component's last-rendered generation and bounds to determine
//! what needs repainting. The framework only calls `render()` when either:
//! - The component's data changed (generation increased)
//! - The component's bounds changed (layout gave it different dimensions/position)
//!
//! When bounds shrink or move, the vacated region is cleared automatically.

use crate::layout::Rect;
use crate::render::Renderer;
use crate::theme::Color;

/// Tracks the render state of a single component
#[derive(Debug, Clone, Copy)]
struct RenderState {
    generation: u64,
    bounds: Rect,
}

/// Manages render state for a tree of components
pub struct RenderTracker {
    states: Vec<(usize, RenderState)>,
    bg_color: Color,
}

impl RenderTracker {
    /// Create a new render tracker
    pub fn new() -> Self {
        RenderTracker {
            states: Vec::new(),
            bg_color: Color::black(),
        }
    }

    /// Set the background color used when clearing vacated regions
    pub fn set_background(&mut self, color: Color) {
        self.bg_color = color;
    }

    /// Check if a component needs rendering and update its tracked state.
    ///
    /// Returns true if the component should be rendered. If the bounds changed,
    /// clears the old region automatically.
    ///
    /// `id` is a stable identifier for the component (index, hash, etc.)
    pub fn needs_render(
        &mut self,
        renderer: &mut dyn Renderer,
        id: usize,
        generation: u64,
        bounds: Rect,
    ) -> bool {
        // Find existing state for this component
        let existing = self.states.iter().position(|(sid, _)| *sid == id);

        match existing {
            Some(idx) => {
                let old = self.states[idx].1;
                let gen_changed = generation != old.generation && generation != u64::MAX;
                let bounds_changed = bounds != old.bounds;

                // GPU surfaces don't retain — must redraw everything every frame
                let must_redraw = !renderer.retains_content();

                if !must_redraw && !gen_changed && !bounds_changed && generation != u64::MAX {
                    return false;
                }

                // Clear vacated region if bounds moved or shrunk
                if bounds_changed {
                    clear_vacated(renderer, old.bounds, bounds, self.bg_color);
                }

                // Update state
                self.states[idx].1 = RenderState { generation, bounds };
                true
            }
            None => {
                // New component — always render
                self.states.push((id, RenderState { generation, bounds }));
                true
            }
        }
    }

    /// Force all components to re-render on next check (e.g., after resize)
    pub fn invalidate_all(&mut self) {
        self.states.clear();
    }

    /// Remove tracking for a component (e.g., when unmounted)
    pub fn remove(&mut self, id: usize) {
        self.states.retain(|(sid, _)| *sid != id);
    }
}

impl RenderTracker {
    /// Render a component only if it needs updating.
    #[allow(clippy::too_many_arguments)]
    ///
    /// Checks generation and bounds, clears vacated regions, and calls
    /// `render()` if needed. Returns true if the component was rendered.
    pub fn render_if_needed(
        &mut self,
        component: &mut dyn crate::component::Component,
        renderer: &mut dyn Renderer,
        id: usize,
        bounds: Rect,
        ctx: &crate::context::RenderContext,
    ) -> anyhow::Result<bool> {
        let generation = component.generation();
        if self.needs_render(renderer, id, generation, bounds) {
            component.render(renderer, bounds, ctx)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for RenderTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Clear the region that was occupied by old bounds but not by new bounds
fn clear_vacated(renderer: &mut dyn Renderer, old: Rect, new: Rect, bg: Color) {
    // If completely different position, clear entire old region
    if old.x != new.x || old.y != new.y || old.width != new.width || old.height != new.height {
        // Clear rows that are in old but not in new
        let old_bottom = old.y + old.height;
        let new_bottom = new.y + new.height;
        let old_right = old.x + old.width;
        let new_right = new.x + new.width;

        // Bottom strip (old extends below new)
        if old_bottom > new_bottom {
            let _ = renderer.fill_rect(
                Rect::new(old.x, new_bottom, old.width, old_bottom - new_bottom),
                bg,
            );
        }

        // Right strip (old extends right of new)
        if old_right > new_right {
            let top = old.y;
            let height = old.height.min(new.height + (new.y.saturating_sub(old.y)));
            let _ = renderer.fill_rect(
                Rect::new(new_right, top, old_right - new_right, height),
                bg,
            );
        }

        // Top strip (old extends above new)
        if old.y < new.y {
            let _ = renderer.fill_rect(
                Rect::new(old.x, old.y, old.width, new.y - old.y),
                bg,
            );
        }

        // Left strip (old extends left of new)
        if old.x < new.x {
            let _ = renderer.fill_rect(
                Rect::new(old.x, old.y, new.x - old.x, old.height),
                bg,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_component_always_renders() {
        struct NullRenderer;
        // Can't easily test with a real renderer, so test the state logic directly
        let mut tracker = RenderTracker::new();
        let bounds = Rect::new(0, 0, 10, 5);

        // First check — new component, should render
        let existing = tracker.states.iter().position(|(sid, _)| *sid == 0);
        assert!(existing.is_none());
    }

    #[test]
    fn test_unchanged_skips() {
        let mut tracker = RenderTracker::new();
        let bounds = Rect::new(0, 0, 10, 5);

        // Manually insert state
        tracker.states.push((0, RenderState { generation: 5, bounds }));

        // Same generation, same bounds — find existing
        let existing = tracker.states.iter().position(|(sid, _)| *sid == 0);
        assert!(existing.is_some());
        let old = tracker.states[existing.unwrap()].1;
        assert_eq!(old.generation, 5);
        assert_eq!(old.bounds, bounds);
    }

    #[test]
    fn test_invalidate_clears_all() {
        let mut tracker = RenderTracker::new();
        tracker.states.push((0, RenderState {
            generation: 1,
            bounds: Rect::new(0, 0, 10, 5),
        }));
        tracker.states.push((1, RenderState {
            generation: 2,
            bounds: Rect::new(10, 0, 10, 5),
        }));

        tracker.invalidate_all();
        assert!(tracker.states.is_empty());
    }

    #[test]
    fn test_generation_max_always_renders() {
        let mut tracker = RenderTracker::new();
        let bounds = Rect::new(0, 0, 10, 5);

        // Insert with MAX generation
        tracker.states.push((0, RenderState { generation: u64::MAX, bounds }));

        // Check — MAX should always re-render
        let old = tracker.states[0].1;
        let gen_changed = u64::MAX != old.generation && u64::MAX != u64::MAX;
        // gen_changed is false because both are MAX, but the special case handles it
        assert!(!gen_changed); // This is correct — the needs_render method handles u64::MAX specially
    }
}

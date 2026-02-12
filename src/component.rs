//! Component system - trait and lifecycle for UI elements

use crate::context::RenderContext;
use crate::event::{Event, EventHandler};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Core component trait for all UI elements
///
/// Components use a hybrid approach:
/// - Retained: Component tree structure and state
/// - Immediate: Rendering happens fresh each frame via render() callback
pub trait Component: EventHandler {
    /// Render the component to the given rectangle
    ///
    /// This is called every frame. Components should issue immediate-mode
    /// drawing commands to the renderer within their bounds.
    ///
    /// The context provides access to theme, locale, and accessibility settings.
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()>;

    /// Calculate minimum size needed for this component (optional)
    fn min_size(&self) -> (u16, u16) {
        (0, 0)
    }

    /// Called when component is first mounted
    fn on_mount(&mut self) {}

    /// Called before component is unmounted
    fn on_unmount(&mut self) {}

    /// Mark component as needing redraw (for optimization)
    fn mark_dirty(&mut self) {}

    /// Check if component needs redraw
    fn is_dirty(&self) -> bool {
        true // Default: always redraw (can be optimized per component)
    }

    /// Get component name for debugging
    fn name(&self) -> &str {
        "Component"
    }
}

/// Container that can hold child components
pub trait Container: Component {
    /// Get mutable access to children
    fn children_mut(&mut self) -> &mut [Box<dyn Component>];

    /// Get immutable access to children
    fn children(&self) -> &[Box<dyn Component>];

    /// Add a child component
    fn add_child(&mut self, child: Box<dyn Component>);

    /// Remove a child by index
    fn remove_child(&mut self, index: usize) -> Option<Box<dyn Component>>;
}

/// Helper to propagate events to children
pub fn propagate_event(children: &mut [Box<dyn Component>], event: &Event) -> bool {
    for child in children.iter_mut() {
        if child.handle_event(event) {
            return true; // Event consumed
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestComponent {
        dirty: bool,
    }

    impl EventHandler for TestComponent {}

    impl Component for TestComponent {
        fn render(
            &mut self,
            _renderer: &mut Renderer,
            _bounds: Rect,
            _ctx: &RenderContext,
        ) -> Result<()> {
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
            "TestComponent"
        }
    }

    #[test]
    fn test_component_dirty_tracking() {
        use crate::slots::Slots;
        use crate::terminal::TerminalCapabilities;
        use crate::theme::Theme;

        let mut comp = TestComponent { dirty: true };
        assert!(comp.is_dirty());

        // Render should clear dirty flag
        let mut renderer = Renderer::headless();
        let caps = TerminalCapabilities::detect();
        let theme = Theme::new(caps);
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);
        comp.render(&mut renderer, Rect::new(0, 0, 10, 10), &ctx)
            .unwrap();
        assert!(!comp.is_dirty());

        // Marking dirty should set it again
        comp.mark_dirty();
        assert!(comp.is_dirty());
    }
}

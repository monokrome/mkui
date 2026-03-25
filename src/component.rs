//! Component system - trait and lifecycle for UI elements

use crate::context::RenderContext;
use crate::event::{Event, EventHandler};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Core component trait for all UI elements
///
/// Components use a hybrid approach:
/// - Retained: Component tree structure and state via `Signal<T>`
/// - Immediate: Rendering happens fresh each frame via render() callback
///
/// Change tracking is automatic through signals. Components expose a
/// `generation()` that reflects the combined generation of their signals.
/// The framework compares this against the last rendered generation to
/// decide whether to call `render()`.
pub trait Component: EventHandler {
    /// Render the component to the given rectangle
    ///
    /// Called by the framework when the component's generation has changed.
    /// Components should issue drawing commands to the renderer within bounds.
    fn render(&mut self, renderer: &mut dyn Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()>;

    /// Calculate minimum size needed for this component (optional)
    fn min_size(&self) -> (u16, u16) {
        (0, 0)
    }

    /// Called when component is first mounted
    fn on_mount(&mut self) {}

    /// Called before component is unmounted
    fn on_unmount(&mut self) {}

    /// Current state generation — changes when component state mutates.
    /// The framework re-renders when this increases.
    /// Default returns u64::MAX so components that don't implement it always render.
    fn generation(&self) -> u64 {
        u64::MAX
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
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Signal;

    struct TestComponent {
        value: Signal<i32>,
    }

    impl EventHandler for TestComponent {}

    impl Component for TestComponent {
        fn render(
            &mut self,
            _renderer: &mut dyn Renderer,
            _bounds: Rect,
            _ctx: &RenderContext,
        ) -> Result<()> {
            Ok(())
        }

        fn generation(&self) -> u64 {
            self.value.generation()
        }

        fn name(&self) -> &str {
            "TestComponent"
        }
    }

    #[test]
    fn test_component_generation_tracking() {
        let mut comp = TestComponent {
            value: Signal::new(0),
        };
        let gen1 = comp.generation();

        comp.value.set(42);
        let gen2 = comp.generation();

        assert!(gen2 > gen1);
    }

    #[test]
    fn test_component_unchanged() {
        let comp = TestComponent {
            value: Signal::new(0),
        };
        let gen1 = comp.generation();
        let gen2 = comp.generation();

        assert_eq!(gen1, gen2);
    }
}

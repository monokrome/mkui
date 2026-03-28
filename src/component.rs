//! Component system - trait and lifecycle for UI elements

use crate::context::RenderContext;
use crate::event::{Event, EventHandler};
use crate::layout::Rect;
use crate::render::Renderer;
use crate::signal::SignalBase;
use anyhow::Result;

/// Core component trait for all UI elements
///
/// Change tracking is automatic through signals. Components implement
/// `signals()` to declare their reactive dependencies. The framework
/// combines signal generations to determine when re-rendering is needed.
///
/// Components that don't implement `signals()` always render (backward compat).
pub trait Component: EventHandler {
    /// Render the component to the given rectangle
    ///
    /// Called by the framework when the component's generation has changed.
    fn render(&mut self, renderer: &mut dyn Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()>;

    /// Declare which signals this component depends on.
    ///
    /// The framework uses these to compute a combined generation automatically.
    /// Default: empty vec (component always renders — backward compatible).
    fn signals(&self) -> Vec<&dyn SignalBase> {
        Vec::new()
    }

    /// Current state generation — changes when component state mutates.
    ///
    /// Default implementation combines signal generations via `signals()`.
    /// Returns `u64::MAX` when `signals()` is empty (always render).
    /// Override this only if you need custom generation logic.
    fn generation(&self) -> u64 {
        let sigs = self.signals();
        if sigs.is_empty() {
            return u64::MAX;
        }
        sigs.iter()
            .map(|s| s.generation())
            .fold(0u64, u64::wrapping_add)
    }

    /// Calculate minimum size needed for this component
    fn min_size(&self) -> (u16, u16) {
        (0, 0)
    }

    /// Called when component is first mounted
    fn on_mount(&mut self) {}

    /// Called before component is unmounted
    fn on_unmount(&mut self) {}

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
        label: Signal<String>,
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

        fn signals(&self) -> Vec<&dyn SignalBase> {
            vec![&self.value, &self.label]
        }

        fn name(&self) -> &str {
            "TestComponent"
        }
    }

    #[test]
    fn test_generation_from_signals() {
        let mut comp = TestComponent {
            value: Signal::new(0),
            label: Signal::new("hello".to_string()),
        };
        let gen1 = comp.generation();

        comp.value.set(42);
        let gen2 = comp.generation();
        assert_ne!(gen1, gen2);

        // No change — same generation
        let gen3 = comp.generation();
        assert_eq!(gen2, gen3);
    }

    #[test]
    fn test_multiple_signal_changes() {
        let mut comp = TestComponent {
            value: Signal::new(0),
            label: Signal::new("a".to_string()),
        };
        let gen1 = comp.generation();

        comp.value.set(1);
        comp.label.set("b".to_string());
        let gen2 = comp.generation();

        assert_ne!(gen1, gen2);
    }

    #[test]
    fn test_no_signals_always_renders() {
        struct AlwaysRender;
        impl EventHandler for AlwaysRender {}
        impl Component for AlwaysRender {
            fn render(&mut self, _: &mut dyn Renderer, _: Rect, _: &RenderContext) -> Result<()> {
                Ok(())
            }
        }

        let comp = AlwaysRender;
        assert_eq!(comp.generation(), u64::MAX);
    }
}

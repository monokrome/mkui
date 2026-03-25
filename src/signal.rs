//! Reactive state primitives for automatic change tracking
//!
//! `Signal<T>` wraps a value and tracks mutations via a generation counter.
//! Components use signals for their mutable state instead of manual dirty flags.
//! The framework compares generations to know which components need re-rendering.

/// Reactive value that tracks mutations via generation counter
///
/// Every call to `set()` increments the generation. The framework compares
/// a component's generation against the last rendered generation to decide
/// whether to re-render.
///
/// ```
/// use mkui::signal::Signal;
///
/// let mut name = Signal::new("hello");
/// assert_eq!(name.generation(), 1);
///
/// name.set("world");
/// assert_eq!(name.generation(), 2);
/// assert_eq!(*name.get(), "world");
/// ```
#[derive(Debug, Clone)]
pub struct Signal<T> {
    value: T,
    generation: u64,
}

impl<T> Signal<T> {
    /// Create a new signal with an initial value (generation starts at 1)
    pub fn new(value: T) -> Self {
        Signal {
            value,
            generation: 1,
        }
    }

    /// Set a new value, incrementing the generation
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.generation += 1;
    }

    /// Get a reference to the current value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference that auto-increments generation on drop
    pub fn get_mut(&mut self) -> SignalGuard<'_, T> {
        SignalGuard {
            value: &mut self.value,
            generation: &mut self.generation,
        }
    }

    /// Get the current generation (monotonically increasing on mutation)
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl<T: PartialEq> Signal<T> {
    /// Set only if the value actually changed
    pub fn set_if_changed(&mut self, value: T) {
        if self.value != value {
            self.value = value;
            self.generation += 1;
        }
    }
}

/// RAII guard that increments generation when the mutable reference is dropped
pub struct SignalGuard<'a, T> {
    value: &'a mut T,
    generation: &'a mut u64,
}

impl<T> std::ops::Deref for SignalGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<T> std::ops::DerefMut for SignalGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T> Drop for SignalGuard<'_, T> {
    fn drop(&mut self) {
        *self.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_new() {
        let s = Signal::new(42);
        assert_eq!(*s.get(), 42);
        assert_eq!(s.generation(), 1);
    }

    #[test]
    fn test_signal_set() {
        let mut s = Signal::new(1);
        s.set(2);
        assert_eq!(*s.get(), 2);
        assert_eq!(s.generation(), 2);
    }

    #[test]
    fn test_signal_set_if_changed() {
        let mut s = Signal::new(1);
        s.set_if_changed(1);
        assert_eq!(s.generation(), 1); // unchanged

        s.set_if_changed(2);
        assert_eq!(s.generation(), 2); // changed
    }

    #[test]
    fn test_signal_get_mut() {
        let mut s = Signal::new(vec![1, 2, 3]);
        assert_eq!(s.generation(), 1);

        {
            let mut guard = s.get_mut();
            guard.push(4);
        } // guard dropped, generation increments

        assert_eq!(s.generation(), 2);
        assert_eq!(s.get().len(), 4);
    }

    #[test]
    fn test_signal_multiple_mutations() {
        let mut s = Signal::new(0);
        for i in 1..=10 {
            s.set(i);
        }
        assert_eq!(s.generation(), 11); // 1 initial + 10 sets
        assert_eq!(*s.get(), 10);
    }
}

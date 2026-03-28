//! Reactive state primitives for automatic change tracking
//!
//! Two primitives for tracking state changes:
//!
//! - `Signal<T>` — single-owner reactive value. Owned by one component.
//! - `Binding<T>` — shared reactive value. Multiple components hold clones
//!   and see the same data. Mutations from any holder are visible to all.
//!
//! Both implement `SignalBase` so the framework can check generation without
//! knowing the concrete type.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Type-erased access to a generation counter.
///
/// The framework uses this to determine when a component needs re-rendering.
pub trait SignalBase {
    /// Current generation — increments on every mutation
    fn generation(&self) -> u64;
}

// ---------------------------------------------------------------------------
// Signal<T> — single-owner reactive value
// ---------------------------------------------------------------------------

/// Single-owner reactive value with generation tracking.
///
/// ```
/// use mkui::signal::Signal;
/// use mkui::signal::SignalBase;
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
    gen: u64,
}

impl<T> Signal<T> {
    /// Create a new signal (generation starts at 1)
    pub fn new(value: T) -> Self {
        Signal { value, gen: 1 }
    }

    /// Set a new value, incrementing the generation
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.gen += 1;
    }

    /// Get a reference to the current value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get a mutable reference that auto-increments generation on drop
    pub fn get_mut(&mut self) -> SignalGuard<'_, T> {
        SignalGuard {
            value: &mut self.value,
            generation: &mut self.gen,
        }
    }
}

impl<T: PartialEq> Signal<T> {
    /// Set only if the value actually changed
    pub fn set_if_changed(&mut self, value: T) {
        if self.value != value {
            self.value = value;
            self.gen += 1;
        }
    }
}

impl<T> SignalBase for Signal<T> {
    fn generation(&self) -> u64 {
        self.gen
    }
}

/// RAII guard that increments generation when dropped
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

// ---------------------------------------------------------------------------
// Binding<T> — shared reactive value (data binding)
// ---------------------------------------------------------------------------

/// Shared reactive value. Clone to share between components.
///
/// Multiple holders see the same data. Mutations from any holder
/// increment the generation and are visible to all others.
///
/// ```
/// use mkui::signal::{Binding, SignalBase};
///
/// let a = Binding::new(42);
/// let b = a.clone();
///
/// assert_eq!(*a.get(), 42);
/// a.set(100);
/// assert_eq!(*b.get(), 100); // b sees the change
/// assert_eq!(a.generation(), b.generation());
/// ```
pub struct Binding<T> {
    inner: Rc<BindingInner<T>>,
}

struct BindingInner<T> {
    value: RefCell<T>,
    gen: Cell<u64>,
}

impl<T> Binding<T> {
    /// Create a new binding (generation starts at 1)
    pub fn new(value: T) -> Self {
        Binding {
            inner: Rc::new(BindingInner {
                value: RefCell::new(value),
                gen: Cell::new(1),
            }),
        }
    }

    /// Set a new value, incrementing the generation
    pub fn set(&self, value: T) {
        *self.inner.value.borrow_mut() = value;
        self.inner.gen.set(self.inner.gen.get() + 1);
    }

    /// Get a shared reference to the current value
    pub fn get(&self) -> std::cell::Ref<'_, T> {
        self.inner.value.borrow()
    }

    /// Get a mutable reference that auto-increments generation on drop
    pub fn get_mut(&self) -> BindingGuard<'_, T> {
        BindingGuard {
            value: self.inner.value.borrow_mut(),
            gen: &self.inner.gen,
        }
    }

    /// Apply a function to the value and increment generation
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.inner.value.borrow_mut());
        self.inner.gen.set(self.inner.gen.get() + 1);
    }
}

impl<T: PartialEq> Binding<T> {
    /// Set only if the value actually changed
    pub fn set_if_changed(&self, value: T) {
        let mut current = self.inner.value.borrow_mut();
        if *current != value {
            *current = value;
            self.inner.gen.set(self.inner.gen.get() + 1);
        }
    }
}

impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Binding {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T> SignalBase for Binding<T> {
    fn generation(&self) -> u64 {
        self.inner.gen.get()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Binding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Binding")
            .field("value", &*self.inner.value.borrow())
            .field("generation", &self.inner.gen.get())
            .finish()
    }
}

/// RAII guard for mutable access to a Binding. Increments generation on drop.
pub struct BindingGuard<'a, T> {
    value: std::cell::RefMut<'a, T>,
    gen: &'a Cell<u64>,
}

impl<T> std::ops::Deref for BindingGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> std::ops::DerefMut for BindingGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> Drop for BindingGuard<'_, T> {
    fn drop(&mut self) {
        self.gen.set(self.gen.get() + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_basic() {
        let mut s = Signal::new(42);
        assert_eq!(*s.get(), 42);
        assert_eq!(s.generation(), 1);

        s.set(100);
        assert_eq!(*s.get(), 100);
        assert_eq!(s.generation(), 2);
    }

    #[test]
    fn test_signal_set_if_changed() {
        let mut s = Signal::new(1);
        s.set_if_changed(1);
        assert_eq!(s.generation(), 1);

        s.set_if_changed(2);
        assert_eq!(s.generation(), 2);
    }

    #[test]
    fn test_signal_get_mut() {
        let mut s = Signal::new(vec![1, 2, 3]);
        {
            let mut guard = s.get_mut();
            guard.push(4);
        }
        assert_eq!(s.generation(), 2);
        assert_eq!(s.get().len(), 4);
    }

    #[test]
    fn test_binding_shared() {
        let a = Binding::new(42);
        let b = a.clone();

        assert_eq!(*a.get(), 42);
        assert_eq!(*b.get(), 42);

        a.set(100);
        assert_eq!(*b.get(), 100);
        assert_eq!(a.generation(), b.generation());
    }

    #[test]
    fn test_binding_generation() {
        let b = Binding::new(0);
        assert_eq!(b.generation(), 1);

        b.set(1);
        assert_eq!(b.generation(), 2);

        b.set(2);
        assert_eq!(b.generation(), 3);
    }

    #[test]
    fn test_binding_get_mut() {
        let b = Binding::new(vec![1, 2]);
        {
            let mut guard = b.get_mut();
            guard.push(3);
        }
        assert_eq!(b.generation(), 2);
        assert_eq!(b.get().len(), 3);
    }

    #[test]
    fn test_binding_update() {
        let b = Binding::new(10);
        b.update(|v| *v += 5);
        assert_eq!(*b.get(), 15);
        assert_eq!(b.generation(), 2);
    }

    #[test]
    fn test_binding_set_if_changed() {
        let b = Binding::new(1);
        b.set_if_changed(1);
        assert_eq!(b.generation(), 1);

        b.set_if_changed(2);
        assert_eq!(b.generation(), 2);
    }

    #[test]
    fn test_signal_multiple_mutations() {
        let mut s = Signal::new(0);
        for i in 1..=10 {
            s.set(i);
        }
        assert_eq!(s.generation(), 11);
        assert_eq!(*s.get(), 10);
    }
}

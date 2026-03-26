//! Rendering context - provides theme, locale, accessibility, and slots to components
//!
//! `RenderContext` carries data down the component tree during rendering.
//! Applications can register custom theme types via `with_extension()` and
//! retrieve them with `extension::<T>()`.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::i18n::{AccessibilitySettings, Locale};
use crate::slots::Slots;
use crate::theme::Theme;

/// Context passed down the component tree during rendering
pub struct RenderContext<'a> {
    /// mkui's built-in theme
    pub theme: &'a Theme,

    /// Locale for formatting and i18n
    pub locale: &'a Locale,

    /// Accessibility settings
    pub accessibility: &'a AccessibilitySettings,

    /// Slot containers for header and status bar
    pub slots: &'a Slots,

    /// Application-specific extensions (custom themes, state, etc.)
    extensions: HashMap<TypeId, &'a dyn Any>,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context from a theme and slots
    pub fn new(theme: &'a Theme, slots: &'a Slots) -> Self {
        RenderContext {
            theme,
            locale: &theme.locale,
            accessibility: &theme.accessibility,
            slots,
            extensions: HashMap::new(),
        }
    }

    /// Register an application-specific extension type
    ///
    /// Any type can be stored and later retrieved by type. Applications use
    /// this for custom themes, configuration, or any data components need.
    ///
    /// ```ignore
    /// struct MyTheme { accent: Color }
    /// let my_theme = MyTheme { accent: Color::rgb(138, 79, 255) };
    /// let ctx = ctx.with_extension(&my_theme);
    ///
    /// // In a component:
    /// if let Some(theme) = ctx.extension::<MyTheme>() {
    ///     renderer.write_styled("hello", &Style::new().fg(theme.accent))?;
    /// }
    /// ```
    pub fn with_extension<T: Any>(mut self, value: &'a T) -> Self {
        self.extensions.insert(TypeId::of::<T>(), value);
        self
    }

    /// Retrieve an application-specific extension by type
    pub fn extension<T: Any>(&self) -> Option<&'a T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    /// Create a child context with a different mkui theme
    pub fn with_theme(&self, theme: &'a Theme) -> Self {
        RenderContext {
            theme,
            locale: &theme.locale,
            accessibility: &theme.accessibility,
            slots: self.slots,
            extensions: self.extensions.clone(),
        }
    }

    /// Create a child context with different slots
    pub fn with_slots(&self, slots: &'a Slots) -> Self {
        RenderContext {
            theme: self.theme,
            locale: self.locale,
            accessibility: self.accessibility,
            slots,
            extensions: self.extensions.clone(),
        }
    }

    /// Create a child context with a different locale
    pub fn with_locale(&self, locale: &'a Locale) -> Self {
        RenderContext {
            theme: self.theme,
            locale,
            accessibility: self.accessibility,
            slots: self.slots,
            extensions: self.extensions.clone(),
        }
    }

    /// Create a child context with different accessibility settings
    pub fn with_accessibility(&self, accessibility: &'a AccessibilitySettings) -> Self {
        RenderContext {
            theme: self.theme,
            locale: self.locale,
            accessibility,
            slots: self.slots,
            extensions: self.extensions.clone(),
        }
    }
}

/// Hook trait for accessing theme from context
pub trait UseTheme {
    /// Get the current mkui theme
    fn use_theme<'a>(&self, ctx: &'a RenderContext) -> &'a Theme {
        ctx.theme
    }

    /// Get text direction from theme
    fn use_text_direction(&self, ctx: &RenderContext) -> crate::i18n::TextDirection {
        ctx.theme.text_direction
    }
}

/// Hook trait for accessing locale from context
pub trait UseLocale {
    /// Get the current locale
    fn use_locale<'a>(&self, ctx: &'a RenderContext) -> &'a Locale {
        ctx.locale
    }

    /// Check if current locale is RTL
    fn use_is_rtl(&self, ctx: &RenderContext) -> bool {
        ctx.locale.text_direction.is_rtl()
    }
}

/// Hook trait for accessing accessibility settings from context
pub trait UseAccessibility {
    /// Get accessibility settings
    fn use_accessibility<'a>(&self, ctx: &'a RenderContext) -> &'a AccessibilitySettings {
        ctx.accessibility
    }

    /// Check if high contrast mode is enabled
    fn use_high_contrast(&self, ctx: &RenderContext) -> bool {
        ctx.accessibility.high_contrast
    }

    /// Get font scale multiplier
    fn use_font_scale(&self, ctx: &RenderContext) -> f32 {
        ctx.accessibility.font_scale
    }

    /// Scale a dimension based on accessibility settings
    fn use_scaled(&self, ctx: &RenderContext, base: u16) -> u16 {
        ctx.accessibility.scale_dimension(base)
    }
}

/// Auto-implement all hook traits for all components
impl<T> UseTheme for T {}
impl<T> UseLocale for T {}
impl<T> UseAccessibility for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let theme = Theme::new();
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);

        assert_eq!(ctx.theme as *const _, &theme as *const _);
        assert_eq!(ctx.locale as *const _, &theme.locale as *const _);
        assert_eq!(ctx.slots as *const _, &slots as *const _);
    }

    #[test]
    fn test_hook_traits() {
        let theme = Theme::new();
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);

        struct TestComponent;

        let component = TestComponent;
        let theme_from_hook = component.use_theme(&ctx);
        assert_eq!(theme_from_hook as *const _, &theme as *const _);

        let locale_from_hook = component.use_locale(&ctx);
        assert_eq!(locale_from_hook as *const _, &theme.locale as *const _);
    }

    struct CustomTheme {
        accent: crate::theme::Color,
    }

    #[test]
    fn test_extension() {
        let theme = Theme::new();
        let slots = Slots::new();
        let custom = CustomTheme {
            accent: crate::theme::Color::rgb(138, 79, 255),
        };

        let ctx = RenderContext::new(&theme, &slots).with_extension(&custom);

        let retrieved = ctx.extension::<CustomTheme>().unwrap();
        assert_eq!(retrieved.accent, crate::theme::Color::Rgb(138, 79, 255));
    }

    #[test]
    fn test_missing_extension() {
        let theme = Theme::new();
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);

        assert!(ctx.extension::<CustomTheme>().is_none());
    }

    #[test]
    fn test_extension_survives_child_context() {
        let theme = Theme::new();
        let slots = Slots::new();
        let slots2 = Slots::new();
        let custom = CustomTheme {
            accent: crate::theme::Color::rgb(100, 200, 50),
        };

        let ctx = RenderContext::new(&theme, &slots).with_extension(&custom);
        let child = ctx.with_slots(&slots2);

        assert!(child.extension::<CustomTheme>().is_some());
    }
}

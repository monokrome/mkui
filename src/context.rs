//! Rendering context - provides theme, locale, accessibility, and slots to components

use crate::i18n::{AccessibilitySettings, Locale};
use crate::slots::Slots;
use crate::theme::Theme;

/// Context passed down the component tree during rendering (like React Context)
#[derive(Clone)]
pub struct RenderContext<'a> {
    /// Current theme
    pub theme: &'a Theme,

    /// Locale for formatting and i18n
    pub locale: &'a Locale,

    /// Accessibility settings
    pub accessibility: &'a AccessibilitySettings,

    /// Slot containers for header and status bar
    pub slots: &'a Slots,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context from a theme and slots
    pub fn new(theme: &'a Theme, slots: &'a Slots) -> Self {
        RenderContext {
            theme,
            locale: &theme.locale,
            accessibility: &theme.accessibility,
            slots,
        }
    }

    /// Create a child context with a different theme (for ThemeProvider)
    pub fn with_theme(&self, theme: &'a Theme) -> Self {
        RenderContext {
            theme,
            locale: &theme.locale,
            accessibility: &theme.accessibility,
            slots: self.slots,
        }
    }

    /// Create a child context with different slots
    pub fn with_slots(&self, slots: &'a Slots) -> Self {
        RenderContext {
            theme: self.theme,
            locale: self.locale,
            accessibility: self.accessibility,
            slots,
        }
    }

    /// Create a child context with a different locale
    pub fn with_locale(&self, locale: &'a Locale) -> Self {
        RenderContext {
            theme: self.theme,
            locale,
            accessibility: self.accessibility,
            slots: self.slots,
        }
    }

    /// Create a child context with different accessibility settings
    pub fn with_accessibility(&self, accessibility: &'a AccessibilitySettings) -> Self {
        RenderContext {
            theme: self.theme,
            locale: self.locale,
            accessibility,
            slots: self.slots,
        }
    }
}

/// Hook trait for accessing theme from context
pub trait UseTheme {
    /// Get the current theme
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
    use crate::terminal::TerminalCapabilities;

    #[test]
    fn test_context_creation() {
        let caps = TerminalCapabilities::detect();
        let theme = Theme::new(caps);
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);

        assert_eq!(ctx.theme as *const _, &theme as *const _);
        assert_eq!(ctx.locale as *const _, &theme.locale as *const _);
        assert_eq!(ctx.slots as *const _, &slots as *const _);
    }

    #[test]
    fn test_hook_traits() {
        let caps = TerminalCapabilities::detect();
        let theme = Theme::new(caps);
        let slots = Slots::new();
        let ctx = RenderContext::new(&theme, &slots);

        // Test hooks (they're auto-implemented for all types via blanket impl)
        struct TestComponent;

        let component = TestComponent;
        let theme_from_hook = component.use_theme(&ctx);
        assert_eq!(theme_from_hook as *const _, &theme as *const _);

        let locale_from_hook = component.use_locale(&ctx);
        assert_eq!(locale_from_hook as *const _, &theme.locale as *const _);
    }
}

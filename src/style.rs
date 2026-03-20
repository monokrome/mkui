//! Type-safe style system for components
//!
//! `Style` represents visual text properties (colors, bold, etc.) and is used
//! directly by the `Renderer` trait. `StyleSheet` provides CSS-like cascading
//! rules that resolve to `Style` values plus layout properties.

use crate::components::text::TextAlign;
use crate::theme::Color;
use std::any::TypeId;

/// Visual style properties for text rendering
///
/// Used by `Renderer::write_styled` to apply colors and text decorations.
/// All fields are `Option` so that `merge()` can distinguish "not set" from "explicitly false".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    /// Foreground (text) color
    pub fg: Option<Color>,
    /// Background color
    pub bg: Option<Color>,
    /// Bold text
    pub bold: Option<bool>,
    /// Dimmed text
    pub dim: Option<bool>,
    /// Italic text
    pub italic: Option<bool>,
    /// Underlined text
    pub underline: Option<bool>,
    /// Reverse video (swap foreground and background)
    pub reverse: Option<bool>,
}

impl Style {
    /// Create a new empty style (no properties set)
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any property is set
    pub fn is_empty(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && self.bold.is_none()
            && self.dim.is_none()
            && self.italic.is_none()
            && self.underline.is_none()
            && self.reverse.is_none()
    }

    /// Set foreground color
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set background color
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set bold
    pub fn bold(mut self, bold: bool) -> Self {
        self.bold = Some(bold);
        self
    }

    /// Set dim
    pub fn dim(mut self, dim: bool) -> Self {
        self.dim = Some(dim);
        self
    }

    /// Set italic
    pub fn italic(mut self, italic: bool) -> Self {
        self.italic = Some(italic);
        self
    }

    /// Set underline
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = Some(underline);
        self
    }

    /// Set reverse video
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = Some(reverse);
        self
    }

    /// Merge another style into this one (other's set properties take precedence)
    pub fn merge(mut self, other: &Style) -> Self {
        if other.fg.is_some() {
            self.fg = other.fg;
        }
        if other.bg.is_some() {
            self.bg = other.bg;
        }
        if other.bold.is_some() {
            self.bold = other.bold;
        }
        if other.dim.is_some() {
            self.dim = other.dim;
        }
        if other.italic.is_some() {
            self.italic = other.italic;
        }
        if other.underline.is_some() {
            self.underline = other.underline;
        }
        if other.reverse.is_some() {
            self.reverse = other.reverse;
        }
        self
    }

    /// Convert to ANSI escape sequence for terminal rendering
    pub fn to_ansi(&self) -> String {
        let mut codes = Vec::new();

        if self.bold == Some(true) {
            codes.push("1".to_string());
        }
        if self.dim == Some(true) {
            codes.push("2".to_string());
        }
        if self.italic == Some(true) {
            codes.push("3".to_string());
        }
        if self.underline == Some(true) {
            codes.push("4".to_string());
        }
        if self.reverse == Some(true) {
            codes.push("7".to_string());
        }

        if let Some(color) = &self.fg {
            codes.push(color_to_ansi_fg(color));
        }
        if let Some(color) = &self.bg {
            codes.push(color_to_ansi_bg(color));
        }

        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
}

/// Convert a Color to an ANSI foreground color code (number portion only)
fn color_to_ansi_fg(color: &Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("38;2;{};{};{}", r, g, b),
        Color::Palette256(idx) => format!("38;5;{}", idx),
        Color::Ansi16(ansi) => ansi.fg_number().to_string(),
        Color::Basic(basic) => basic.fg_number().to_string(),
    }
}

/// Convert a Color to an ANSI background color code (number portion only)
fn color_to_ansi_bg(color: &Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("48;2;{};{};{}", r, g, b),
        Color::Palette256(idx) => format!("48;5;{}", idx),
        Color::Ansi16(ansi) => ansi.bg_number().to_string(),
        Color::Basic(basic) => basic.bg_number().to_string(),
    }
}

/// A layout/style property for the stylesheet system
#[derive(Debug, Clone)]
pub enum StyleProperty {
    /// Visual style (colors, bold, etc.)
    Visual(Style),
    /// Text alignment
    TextAlign(TextAlign),
    /// Padding (in cells)
    Padding(u16),
    /// Gap between children (in cells)
    Gap(u16),
}

/// Selector for matching components
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selector {
    /// Match all components of a specific type (type-safe)
    Type(TypeId),
    /// Match by component name (string-based, less safe but flexible)
    Name(&'static str),
    /// Match by custom class (components can opt-in to classes)
    Class(&'static str),
    /// Match by unique ID
    Id(&'static str),
}

/// A style rule that applies to matching components
#[derive(Debug, Clone)]
pub struct StyleRule {
    selector: Selector,
    style: Style,
    /// Priority (higher = more important, default = 0)
    priority: u16,
}

impl StyleRule {
    /// Create a new style rule
    pub fn new(selector: Selector, style: Style) -> Self {
        StyleRule {
            selector,
            style,
            priority: 0,
        }
    }

    /// Set the priority of this rule
    pub fn with_priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }

    /// Get the selector
    pub fn selector(&self) -> &Selector {
        &self.selector
    }

    /// Get the style
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Get the priority
    pub fn priority(&self) -> u16 {
        self.priority
    }
}

/// A collection of style rules (like a stylesheet)
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    rules: Vec<StyleRule>,
}

impl StyleSheet {
    /// Create a new empty stylesheet
    pub fn new() -> Self {
        StyleSheet { rules: Vec::new() }
    }

    /// Add a rule to the stylesheet
    pub fn add_rule(mut self, rule: StyleRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add a type-safe rule for all components of a given type
    pub fn style_type<T: 'static>(self, style: Style) -> Self {
        self.add_rule(StyleRule::new(Selector::Type(TypeId::of::<T>()), style))
    }

    /// Add a rule for components with a specific name
    pub fn style_name(self, name: &'static str, style: Style) -> Self {
        self.add_rule(StyleRule::new(Selector::Name(name), style))
    }

    /// Add a rule for components with a specific class
    pub fn style_class(self, class: &'static str, style: Style) -> Self {
        self.add_rule(StyleRule::new(Selector::Class(class), style))
    }

    /// Add a rule for a component with a specific ID
    pub fn style_id(self, id: &'static str, style: Style) -> Self {
        self.add_rule(StyleRule::new(Selector::Id(id), style))
    }

    /// Get all matching styles for a given selector, sorted by priority
    pub fn get_styles(&self, selector: &Selector) -> Vec<&Style> {
        let mut matching: Vec<_> = self
            .rules
            .iter()
            .filter(|rule| &rule.selector == selector)
            .collect();

        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        matching.iter().map(|rule| &rule.style).collect()
    }

    /// Compute the final style for a component by merging all matching rules
    pub fn compute_style(&self, selectors: &[Selector]) -> Style {
        let mut final_style = Style::new();

        let mut all_rules: Vec<_> = selectors
            .iter()
            .flat_map(|selector| {
                self.rules
                    .iter()
                    .filter(move |rule| &rule.selector == selector)
            })
            .collect();

        all_rules.sort_by_key(|rule| rule.priority);

        for rule in all_rules {
            final_style = final_style.merge(&rule.style);
        }

        final_style
    }
}

/// Trait for components that support styling
pub trait Styleable: 'static {
    /// Get the type selector for this component
    fn type_selector(&self) -> Selector {
        Selector::Type(TypeId::of::<Self>())
    }

    /// Get the name selector (if any)
    fn name_selector(&self) -> Option<Selector> {
        None
    }

    /// Get class selectors (if any)
    fn class_selectors(&self) -> Vec<Selector> {
        Vec::new()
    }

    /// Get ID selector (if any)
    fn id_selector(&self) -> Option<Selector> {
        None
    }

    /// Get all selectors for this component
    fn selectors(&self) -> Vec<Selector> {
        let mut selectors = vec![self.type_selector()];

        if let Some(name) = self.name_selector() {
            selectors.push(name);
        }

        selectors.extend(self.class_selectors());

        if let Some(id) = self.id_selector() {
            selectors.push(id);
        }

        selectors
    }

    /// Compute the final style for this component from a stylesheet
    fn compute_style(&self, stylesheet: &StyleSheet) -> Style {
        stylesheet.compute_style(&self.selectors())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::text::Text;

    #[test]
    fn test_style_creation() {
        let style = Style::new().bold(true);

        assert_eq!(style.bold, Some(true));
        assert_eq!(style.fg, None);
    }

    #[test]
    fn test_style_merge() {
        let style1 = Style::new().bold(true).fg(Color::Rgb(255, 0, 0));
        let style2 = Style::new().dim(true).fg(Color::Rgb(0, 255, 0));

        let merged = style1.merge(&style2);

        assert_eq!(merged.bold, Some(true));
        assert_eq!(merged.dim, Some(true));
        assert_eq!(merged.fg, Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_stylesheet_type_selector() {
        let stylesheet = StyleSheet::new().style_type::<Text>(Style::new().bold(true));

        let selector = Selector::Type(TypeId::of::<Text>());
        let styles = stylesheet.get_styles(&selector);

        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].bold, Some(true));
    }

    #[test]
    fn test_stylesheet_priority() {
        let stylesheet = StyleSheet::new()
            .add_rule(
                StyleRule::new(Selector::Name("test"), Style::new().fg(Color::Rgb(255, 0, 0)))
                    .with_priority(1),
            )
            .add_rule(
                StyleRule::new(Selector::Name("test"), Style::new().fg(Color::Rgb(0, 255, 0)))
                    .with_priority(10),
            );

        let final_style = stylesheet.compute_style(&[Selector::Name("test")]);

        assert_eq!(final_style.fg, Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_empty_stylesheet() {
        let stylesheet = StyleSheet::new();
        let style = stylesheet.compute_style(&[Selector::Name("test")]);

        assert!(style.is_empty());
    }

    #[test]
    fn test_no_matching_selector() {
        let stylesheet =
            StyleSheet::new().style_name("foo", Style::new().fg(Color::Rgb(255, 0, 0)));

        let style = stylesheet.compute_style(&[Selector::Name("bar")]);

        assert!(style.is_empty());
    }

    #[test]
    fn test_style_multiple_selectors() {
        let stylesheet = StyleSheet::new()
            .style_name("foo", Style::new().fg(Color::Rgb(255, 0, 0)))
            .style_class("bar", Style::new().bold(true));

        let style = stylesheet.compute_style(&[Selector::Name("foo"), Selector::Class("bar")]);

        assert_eq!(style.fg, Some(Color::Rgb(255, 0, 0)));
        assert_eq!(style.bold, Some(true));
    }

    #[test]
    fn test_style_property_override() {
        let stylesheet = StyleSheet::new()
            .add_rule(
                StyleRule::new(
                    Selector::Name("test"),
                    Style::new().fg(Color::Rgb(255, 0, 0)),
                )
                .with_priority(1),
            )
            .add_rule(
                StyleRule::new(
                    Selector::Class("test"),
                    Style::new().fg(Color::Rgb(0, 255, 0)),
                )
                .with_priority(5),
            );

        let style = stylesheet.compute_style(&[Selector::Name("test"), Selector::Class("test")]);

        assert_eq!(style.fg, Some(Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_style_to_ansi() {
        let style = Style::new().bold(true).reverse(true);
        assert_eq!(style.to_ansi(), "\x1b[1;7m");

        let empty = Style::new();
        assert_eq!(empty.to_ansi(), "");

        let color = Style::new().fg(Color::Rgb(255, 0, 0));
        assert_eq!(color.to_ansi(), "\x1b[38;2;255;0;0m");
    }
}

//! Type-safe style system for components
//!
//! Provides a way to define and apply styles to components in a type-safe manner,
//! similar to CSS but with Rust's type system guarantees.

use crate::components::text::TextAlign;
use crate::theme::Color;
use std::any::TypeId;
use std::collections::HashMap;

/// A style property that can be applied to components
#[derive(Debug, Clone)]
pub enum StyleProperty {
    /// Text color
    Color(Color),
    /// Background color
    Background(Color),
    /// Text alignment
    TextAlign(TextAlign),
    /// Padding (in cells)
    Padding(u16),
    /// Gap between children (in cells)
    Gap(u16),
    /// Whether text should be bold
    Bold(bool),
    /// Whether text should be dimmed
    Dim(bool),
    /// Whether text should be italic
    Italic(bool),
    /// Whether text should be underlined
    Underline(bool),
}

/// A collection of style properties
#[derive(Debug, Clone, Default)]
pub struct Style {
    properties: HashMap<&'static str, StyleProperty>,
}

impl Style {
    /// Create a new empty style
    pub fn new() -> Self {
        Style {
            properties: HashMap::new(),
        }
    }

    /// Set a color property
    pub fn color(mut self, color: Color) -> Self {
        self.properties.insert("color", StyleProperty::Color(color));
        self
    }

    /// Set a background color
    pub fn background(mut self, color: Color) -> Self {
        self.properties
            .insert("background", StyleProperty::Background(color));
        self
    }

    /// Set text alignment
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.properties
            .insert("text_align", StyleProperty::TextAlign(align));
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: u16) -> Self {
        self.properties
            .insert("padding", StyleProperty::Padding(padding));
        self
    }

    /// Set gap
    pub fn gap(mut self, gap: u16) -> Self {
        self.properties.insert("gap", StyleProperty::Gap(gap));
        self
    }

    /// Set bold
    pub fn bold(mut self, bold: bool) -> Self {
        self.properties.insert("bold", StyleProperty::Bold(bold));
        self
    }

    /// Set dim
    pub fn dim(mut self, dim: bool) -> Self {
        self.properties.insert("dim", StyleProperty::Dim(dim));
        self
    }

    /// Set italic
    pub fn italic(mut self, italic: bool) -> Self {
        self.properties
            .insert("italic", StyleProperty::Italic(italic));
        self
    }

    /// Set underline
    pub fn underline(mut self, underline: bool) -> Self {
        self.properties
            .insert("underline", StyleProperty::Underline(underline));
        self
    }

    /// Get a property by key
    pub fn get(&self, key: &str) -> Option<&StyleProperty> {
        self.properties.get(key)
    }

    /// Check if style has a property
    pub fn has(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Merge another style into this one (other takes precedence)
    pub fn merge(mut self, other: &Style) -> Self {
        for (key, value) in &other.properties {
            self.properties.insert(key, value.clone());
        }
        self
    }
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

        // Sort by priority (highest first)
        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        matching.iter().map(|rule| &rule.style).collect()
    }

    /// Compute the final style for a component by merging all matching rules
    pub fn compute_style(&self, selectors: &[Selector]) -> Style {
        let mut final_style = Style::new();

        // Collect all matching styles from all selectors
        let mut all_rules: Vec<_> = selectors
            .iter()
            .flat_map(|selector| {
                self.rules
                    .iter()
                    .filter(move |rule| &rule.selector == selector)
            })
            .collect();

        // Sort by priority (lowest first, so higher priority overrides)
        all_rules.sort_by_key(|rule| rule.priority);

        // Merge styles in order of priority
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
        let style = Style::new().bold(true).padding(2);

        assert!(style.has("bold"));
        assert!(style.has("padding"));
        assert!(!style.has("color"));
    }

    #[test]
    fn test_style_merge() {
        let style1 = Style::new().bold(true).padding(2);
        let style2 = Style::new().padding(4).dim(true);

        let merged = style1.merge(&style2);

        // style2's padding should override
        if let Some(StyleProperty::Padding(p)) = merged.get("padding") {
            assert_eq!(*p, 4);
        } else {
            panic!("Expected padding property");
        }

        // Both bold and dim should be present
        assert!(merged.has("bold"));
        assert!(merged.has("dim"));
    }

    #[test]
    fn test_stylesheet_type_selector() {
        let stylesheet = StyleSheet::new().style_type::<Text>(Style::new().bold(true));

        let selector = Selector::Type(TypeId::of::<Text>());
        let styles = stylesheet.get_styles(&selector);

        assert_eq!(styles.len(), 1);
        assert!(styles[0].has("bold"));
    }

    #[test]
    fn test_stylesheet_priority() {
        let stylesheet = StyleSheet::new()
            .add_rule(
                StyleRule::new(Selector::Name("test"), Style::new().padding(2)).with_priority(1),
            )
            .add_rule(
                StyleRule::new(Selector::Name("test"), Style::new().padding(4)).with_priority(10),
            );

        let final_style = stylesheet.compute_style(&[Selector::Name("test")]);

        // Higher priority (10) should win
        if let Some(StyleProperty::Padding(p)) = final_style.get("padding") {
            assert_eq!(*p, 4);
        } else {
            panic!("Expected padding property");
        }
    }

    #[test]
    fn test_empty_stylesheet() {
        let stylesheet = StyleSheet::new();
        let style = stylesheet.compute_style(&[Selector::Name("test")]);

        // Empty stylesheet should produce empty style
        assert!(!style.has("padding"));
        assert!(!style.has("color"));
    }

    #[test]
    fn test_no_matching_selector() {
        let stylesheet = StyleSheet::new().style_name("foo", Style::new().padding(2));

        // Query for a selector that doesn't exist
        let style = stylesheet.compute_style(&[Selector::Name("bar")]);

        assert!(!style.has("padding"));
    }

    #[test]
    fn test_style_multiple_selectors() {
        let stylesheet = StyleSheet::new()
            .style_name("foo", Style::new().padding(2))
            .style_class("bar", Style::new().bold(true));

        // Component matches both selectors
        let style = stylesheet.compute_style(&[Selector::Name("foo"), Selector::Class("bar")]);

        // Should have properties from both
        assert!(style.has("padding"));
        assert!(style.has("bold"));
    }

    #[test]
    fn test_style_property_override() {
        let stylesheet = StyleSheet::new()
            .add_rule(
                StyleRule::new(Selector::Name("test"), Style::new().padding(2)).with_priority(1),
            )
            .add_rule(
                StyleRule::new(Selector::Class("test"), Style::new().padding(10)).with_priority(5),
            );

        // Query with both selectors - class has higher priority
        let style = stylesheet.compute_style(&[Selector::Name("test"), Selector::Class("test")]);

        if let Some(StyleProperty::Padding(p)) = style.get("padding") {
            assert_eq!(*p, 10); // Higher priority wins
        } else {
            panic!("Expected padding property");
        }
    }
}

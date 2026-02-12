//! Internationalization and localization support

/// Text reading direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl TextDirection {
    /// Detect text direction from language code
    pub fn from_lang(lang: &str) -> Self {
        // RTL languages
        if lang.starts_with("ar") ||  // Arabic
           lang.starts_with("he") ||  // Hebrew
           lang.starts_with("fa") ||  // Persian/Farsi
           lang.starts_with("ur") ||  // Urdu
           lang.starts_with("yi")
        // Yiddish
        {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        }
    }

    /// Check if this is RTL
    pub fn is_rtl(&self) -> bool {
        matches!(self, TextDirection::RightToLeft)
    }
}

/// Locale information for formatting and display
#[derive(Debug, Clone)]
pub struct Locale {
    /// Language code (ISO 639-1): "en", "ar", "he", "ja", etc.
    pub language: String,

    /// Optional region code (ISO 3166-1): "US", "GB", "SA", "IL", etc.
    pub region: Option<String>,

    /// Text direction for this locale
    pub text_direction: TextDirection,

    /// Decimal separator ('.' or ',')
    pub decimal_separator: char,

    /// Thousands separator (',' or '.' or ' ')
    pub thousands_separator: char,

    /// Date format string (e.g., "%Y-%m-%d", "%d/%m/%Y")
    pub date_format: String,

    /// Time format string (e.g., "%H:%M:%S", "%I:%M %p")
    pub time_format: String,

    /// Currency symbol
    pub currency_symbol: String,

    /// Currency position (before or after amount)
    pub currency_before: bool,
}

impl Locale {
    /// Create a new locale from language and optional region
    pub fn new(language: impl Into<String>, region: Option<String>) -> Self {
        let language = language.into();
        let text_direction = TextDirection::from_lang(&language);

        // Set defaults based on common patterns
        let (decimal_sep, thousands_sep) = match language.as_str() {
            "de" | "es" | "fr" | "it" | "pt" | "ru" => (',', '.'),
            _ => ('.', ','),
        };

        let date_format = match language.as_str() {
            "en" if region.as_deref() == Some("US") => "%m/%d/%Y".to_string(),
            "en" => "%d/%m/%Y".to_string(),
            "ja" | "zh" | "ko" => "%Y-%m-%d".to_string(),
            _ => "%d.%m.%Y".to_string(),
        };

        let time_format = match language.as_str() {
            "en" if region.as_deref() == Some("US") => "%I:%M %p".to_string(),
            _ => "%H:%M".to_string(),
        };

        let currency_symbol = match (language.as_str(), region.as_deref()) {
            ("en", Some("US")) => "$".to_string(),
            ("en", Some("GB")) => "£".to_string(),
            ("ja", _) => "¥".to_string(),
            ("ar", Some("SA")) => "﷼".to_string(),
            _ => "$".to_string(),
        };

        Locale {
            language,
            region,
            text_direction,
            decimal_separator: decimal_sep,
            thousands_separator: thousands_sep,
            date_format,
            time_format,
            currency_symbol,
            currency_before: true,
        }
    }

    /// Parse locale from string like "en-US", "ar-SA", "he-IL"
    pub fn from_string(locale_str: &str) -> Self {
        // Handle empty string
        if locale_str.is_empty() {
            return Self::default();
        }

        let parts: Vec<&str> = locale_str.split('-').collect();
        let language = parts
            .first()
            .filter(|s| !s.is_empty())
            .unwrap_or(&"en")
            .to_string();
        let region = parts.get(1).map(|s| s.to_uppercase());

        Self::new(language, region)
    }

    /// Detect locale from environment
    pub fn from_env() -> Self {
        if let Ok(lang) = std::env::var("LANG") {
            // LANG is typically like "en_US.UTF-8" or "ar_SA.UTF-8"
            let locale_part = lang.split('.').next().unwrap_or("en_US");
            let normalized = locale_part.replace('_', "-");
            Self::from_string(&normalized)
        } else {
            Self::default()
        }
    }

    /// Format a number with locale-specific separators
    pub fn format_number(&self, num: f64, decimals: usize) -> String {
        let abs_num = num.abs();
        let integer_part = abs_num.trunc() as i64;
        let fractional_part = abs_num.fract();

        // Format integer part with thousands separator
        let int_str = integer_part.to_string();
        let mut formatted_int = String::new();

        for (i, ch) in int_str.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                formatted_int.insert(0, self.thousands_separator);
            }
            formatted_int.insert(0, ch);
        }

        // Add decimal part if needed
        let result = if decimals > 0 {
            // Multiply fractional part to get the desired decimal places
            let multiplier = 10_f64.powi(decimals as i32);
            let decimal_value = (fractional_part * multiplier).round() as u64;
            let decimal_str = format!("{:0width$}", decimal_value, width = decimals);
            format!("{}{}{}", formatted_int, self.decimal_separator, decimal_str)
        } else {
            formatted_int
        };

        // Add sign
        if num < 0.0 {
            format!("-{}", result)
        } else {
            result
        }
    }

    /// Format currency
    pub fn format_currency(&self, amount: f64) -> String {
        let formatted = self.format_number(amount, 2);
        if self.currency_before {
            format!("{}{}", self.currency_symbol, formatted)
        } else {
            format!("{} {}", formatted, self.currency_symbol)
        }
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::new("en", Some("US".to_string()))
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref region) = self.region {
            write!(f, "{}-{}", self.language, region)
        } else {
            write!(f, "{}", self.language)
        }
    }
}

/// Accessibility settings
#[derive(Debug, Clone)]
pub struct AccessibilitySettings {
    /// Use high contrast colors
    pub high_contrast: bool,

    /// Reduce or disable animations
    pub prefer_reduced_motion: bool,

    /// Screen reader is active
    pub screen_reader_enabled: bool,

    /// Font scale multiplier (1.0 = normal, 1.5 = 150%, etc.)
    pub font_scale: f32,
}

impl AccessibilitySettings {
    /// Create default accessibility settings
    pub fn new() -> Self {
        Self {
            high_contrast: false,
            prefer_reduced_motion: false,
            screen_reader_enabled: false,
            font_scale: 1.0,
        }
    }

    /// Detect accessibility settings from environment
    pub fn from_env() -> Self {
        Self {
            high_contrast: std::env::var("ACCESSIBILITY_HIGH_CONTRAST").is_ok(),
            prefer_reduced_motion: std::env::var("ACCESSIBILITY_REDUCED_MOTION").is_ok(),
            screen_reader_enabled: std::env::var("SCREEN_READER").is_ok(),
            font_scale: std::env::var("FONT_SCALE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
        }
    }

    /// Apply font scaling to a dimension
    pub fn scale_dimension(&self, base: u16) -> u16 {
        (base as f32 * self.font_scale).round() as u16
    }
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Accessibility role for components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityRole {
    None,
    Button,
    Heading { level: u8 },
    Link,
    List,
    ListItem,
    TextBox,
    Label,
    StatusBar,
    Menu,
    MenuItem,
    Dialog,
    Alert,
    ProgressBar,
    Tab,
    TabPanel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_direction_detection() {
        assert_eq!(TextDirection::from_lang("en"), TextDirection::LeftToRight);
        assert_eq!(TextDirection::from_lang("ar"), TextDirection::RightToLeft);
        assert_eq!(TextDirection::from_lang("he"), TextDirection::RightToLeft);
        assert_eq!(TextDirection::from_lang("ja"), TextDirection::LeftToRight);
    }

    #[test]
    fn test_locale_parsing() {
        let locale = Locale::from_string("en-US");
        assert_eq!(locale.language, "en");
        assert_eq!(locale.region, Some("US".to_string()));
        assert_eq!(locale.text_direction, TextDirection::LeftToRight);

        let locale_ar = Locale::from_string("ar-SA");
        assert_eq!(locale_ar.text_direction, TextDirection::RightToLeft);
    }

    #[test]
    fn test_number_formatting() {
        let locale_us = Locale::new("en", Some("US".to_string()));
        assert_eq!(locale_us.format_number(1234.56, 2), "1,234.56");

        let locale_de = Locale::new("de", Some("DE".to_string()));
        assert_eq!(locale_de.format_number(1234.56, 2), "1.234,56");
    }

    #[test]
    fn test_currency_formatting() {
        let locale_us = Locale::new("en", Some("US".to_string()));
        let formatted = locale_us.format_currency(1234.56);
        assert!(formatted.starts_with('$'));
        assert!(formatted.contains("1,234.56"));
    }

    #[test]
    fn test_accessibility_scaling() {
        let mut settings = AccessibilitySettings::new();
        settings.font_scale = 1.5;

        assert_eq!(settings.scale_dimension(10), 15);
        assert_eq!(settings.scale_dimension(20), 30);
    }

    #[test]
    fn test_number_formatting_edge_cases() {
        let locale_us = Locale::new("en", Some("US".to_string()));

        // Zero
        assert_eq!(locale_us.format_number(0.0, 2), "0.00");

        // Negative numbers
        assert_eq!(locale_us.format_number(-1234.56, 2), "-1,234.56");

        // No decimals
        assert_eq!(locale_us.format_number(1234.0, 0), "1,234");

        // Very small number
        assert_eq!(locale_us.format_number(0.01, 2), "0.01");
    }

    #[test]
    fn test_locale_from_invalid_string() {
        // Should not panic, just use defaults
        let locale = Locale::from_string("");
        assert_eq!(locale.language, "en");

        // Malformed but should handle gracefully
        let locale2 = Locale::from_string("xyz");
        assert_eq!(locale2.language, "xyz");
        assert_eq!(locale2.region, None);
    }

    #[test]
    fn test_text_direction_unknown_language() {
        // Unknown languages should default to LTR
        assert_eq!(TextDirection::from_lang("xyz"), TextDirection::LeftToRight);
        assert_eq!(TextDirection::from_lang(""), TextDirection::LeftToRight);
    }

    #[test]
    fn test_accessibility_scaling_edge_cases() {
        let settings = AccessibilitySettings::new();

        // Zero input (edge case)
        assert_eq!(settings.scale_dimension(0), 0);

        // Very large input
        let large_val = settings.scale_dimension(1000);
        assert_eq!(large_val, 1000); // No scaling with font_scale = 1.0

        // Scaling with extreme values
        let mut extreme = AccessibilitySettings::new();
        extreme.font_scale = 3.0;
        assert_eq!(extreme.scale_dimension(10), 30);
    }
}

//! Theming system with automatic color degradation

mod color;

pub use color::{AnsiColor, BasicColor, Color};

use crate::i18n::{AccessibilitySettings, Locale, TextDirection};

/// Border style for components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// No visible border
    None,
    /// Thin single-line border (─│┌┐└┘)
    Single,
    /// Double-line border (═║╔╗╚╝)
    Double,
    /// Single-line border with rounded corners (╭╮╰╯)
    Rounded,
    /// Thick single-line border (━┃┏┓┗┛)
    Heavy,
    /// ASCII-only border (-|++++)
    Ascii,
}

/// Theme defining colors, spacing, typography for UI components
#[derive(Debug, Clone)]
pub struct Theme {
    /// Default text foreground color
    pub text_fg: Color,
    /// Heading foreground color
    pub heading_fg: Color,
    /// Label foreground color
    pub label_fg: Color,
    /// Error text foreground color
    pub error_fg: Color,
    /// Success text foreground color
    pub success_fg: Color,
    /// Warning text foreground color
    pub warning_fg: Color,
    /// Hyperlink foreground color
    pub link_fg: Color,

    /// Base background color
    pub background: Color,
    /// Surface background color (e.g., cards, panels)
    pub surface: Color,
    /// Elevated surface background color (e.g., modals, popovers)
    pub surface_elevated: Color,

    /// Header title foreground color
    pub header_title_fg: Color,
    /// Header background color, if any
    pub header_bg: Option<Color>,

    /// Badge background color
    pub badge_bg: Color,
    /// Badge text foreground color
    pub badge_fg: Color,

    /// Status bar foreground color
    pub status_fg: Color,
    /// Status bar background color, if any
    pub status_bg: Option<Color>,

    /// Default border color
    pub border_color: Color,
    /// Border color for focused elements
    pub focus_border_color: Color,

    /// Extra-small spacing unit
    pub spacing_xs: u16,
    /// Small spacing unit
    pub spacing_sm: u16,
    /// Medium spacing unit
    pub spacing_md: u16,
    /// Large spacing unit
    pub spacing_lg: u16,
    /// Extra-large spacing unit
    pub spacing_xl: u16,

    /// Default gap between elements
    pub default_gap: u16,
    /// Default padding within elements
    pub default_padding: u16,

    /// Font scaling factor (1.0 = normal)
    pub font_scale: f32,
    /// Line height multiplier
    pub line_height: f32,
    /// Whether headings render in bold
    pub heading_bold: bool,
    /// Whether labels render with dim attribute
    pub label_dim: bool,

    /// Border drawing style
    pub border_style: BorderStyle,
    /// Border width in cells
    pub border_width: u16,

    /// Text layout direction (LTR or RTL)
    pub text_direction: TextDirection,
    /// Active locale for the theme
    pub locale: Locale,

    /// Accessibility overrides (font scale, contrast, etc.)
    pub accessibility: AccessibilitySettings,

}

impl Theme {
    /// Create a new theme with default colors
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let locale = Locale::from_env();
        let text_direction = TextDirection::from_lang(&locale.language);

        Theme {
            text_fg: Color::white(),
            heading_fg: Color::white(),
            label_fg: Color::dark_gray(),
            error_fg: Color::rgb(255, 100, 100),
            success_fg: Color::rgb(100, 255, 100),
            warning_fg: Color::rgb(255, 200, 100),
            link_fg: Color::rgb(100, 150, 255),

            background: Color::black(),
            surface: Color::rgb(20, 20, 25),
            surface_elevated: Color::rgb(30, 30, 35),

            header_title_fg: Color::white(),
            header_bg: None,

            badge_bg: Color::white(),
            badge_fg: Color::black(),

            status_fg: Color::white(),
            status_bg: Some(Color::dark_purple()),

            border_color: Color::dark_gray(),
            focus_border_color: Color::rgb(100, 150, 255),

            spacing_xs: 1,
            spacing_sm: 2,
            spacing_md: 4,
            spacing_lg: 8,
            spacing_xl: 16,

            default_gap: 2,
            default_padding: 1,

            font_scale: 1.0,
            line_height: 1.2,
            heading_bold: true,
            label_dim: true,

            border_style: BorderStyle::None,
            border_width: 1,

            text_direction,
            locale,

            accessibility: AccessibilitySettings::from_env(),
        }
    }

    /// Style for bold header title text
    pub fn header_title_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.header_title_fg).bold(true)
    }

    /// Style for badge foreground and background
    pub fn badge_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.badge_fg).bg(self.badge_bg)
    }

    /// Style for status bar text and background
    pub fn status_style(&self) -> crate::style::Style {
        let style = crate::style::Style::new().fg(self.status_fg);
        if let Some(bg) = self.status_bg {
            style.bg(bg)
        } else {
            style.reverse(true)
        }
    }

    /// Style for status bar background fill only
    pub fn status_bg_fill(&self) -> crate::style::Style {
        if let Some(bg) = self.status_bg {
            crate::style::Style::new().bg(bg)
        } else {
            crate::style::Style::new().reverse(true)
        }
    }

    /// Style for default text color
    pub fn text_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.text_fg)
    }

    /// Style for heading text, optionally bold
    pub fn heading_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.heading_fg).bold(self.heading_bold)
    }

    /// Style for label text, optionally dim
    pub fn label_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.label_fg).dim(self.label_dim)
    }

    /// Style for error text color
    pub fn error_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.error_fg)
    }

    /// Style for success text color
    pub fn success_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.success_fg)
    }

    /// Style for warning text color
    pub fn warning_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.warning_fg)
    }

    /// Style for underlined link text
    pub fn link_style(&self) -> crate::style::Style {
        crate::style::Style::new().fg(self.link_fg).underline(true)
    }

    /// Style for base background color
    pub fn background_style(&self) -> crate::style::Style {
        crate::style::Style::new().bg(self.background)
    }

    /// Style for surface background color
    pub fn surface_style(&self) -> crate::style::Style {
        crate::style::Style::new().bg(self.surface)
    }

    /// Style for elevated surface background color
    pub fn surface_elevated_style(&self) -> crate::style::Style {
        crate::style::Style::new().bg(self.surface_elevated)
    }

    /// Apply font scaling to a dimension
    pub fn scale(&self, base: u16) -> u16 {
        let scaled = (base as f32 * self.font_scale * self.accessibility.font_scale).round() as u16;
        scaled.max(1)
    }

    /// Get border characters for current border style
    pub fn border_chars(&self) -> BorderChars {
        match self.border_style {
            BorderStyle::None => BorderChars::none(),
            BorderStyle::Single => BorderChars::single(),
            BorderStyle::Double => BorderChars::double(),
            BorderStyle::Rounded => BorderChars::rounded(),
            BorderStyle::Heavy => BorderChars::heavy(),
            BorderStyle::Ascii => BorderChars::ascii(),
        }
    }
}

/// Border characters for drawing boxes
#[derive(Debug, Clone)]
pub struct BorderChars {
    /// Horizontal line character
    pub horizontal: char,
    /// Vertical line character
    pub vertical: char,
    /// Top-left corner character
    pub top_left: char,
    /// Top-right corner character
    pub top_right: char,
    /// Bottom-left corner character
    pub bottom_left: char,
    /// Bottom-right corner character
    pub bottom_right: char,
}

impl BorderChars {
    /// Space characters (invisible border)
    pub fn none() -> Self {
        BorderChars {
            horizontal: ' ',
            vertical: ' ',
            top_left: ' ',
            top_right: ' ',
            bottom_left: ' ',
            bottom_right: ' ',
        }
    }

    /// Thin single-line box-drawing characters
    pub fn single() -> Self {
        BorderChars {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
        }
    }

    /// Double-line box-drawing characters
    pub fn double() -> Self {
        BorderChars {
            horizontal: '═',
            vertical: '║',
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
        }
    }

    /// Single-line box-drawing characters with rounded corners
    pub fn rounded() -> Self {
        BorderChars {
            horizontal: '─',
            vertical: '│',
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
        }
    }

    /// Thick single-line box-drawing characters
    pub fn heavy() -> Self {
        BorderChars {
            horizontal: '━',
            vertical: '┃',
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
        }
    }

    /// ASCII-only fallback characters (-|+)
    pub fn ascii() -> Self {
        BorderChars {
            horizontal: '-',
            vertical: '|',
            top_left: '+',
            top_right: '+',
            bottom_left: '+',
            bottom_right: '+',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = Theme::new();

        let style = theme.header_title_style();
        assert!(!style.is_empty());
    }
}

//! Theming system with automatic color degradation

mod color;

pub use color::{AnsiColor, BasicColor, Color};

use crate::i18n::{AccessibilitySettings, Locale, TextDirection};
use crate::terminal::TerminalCapabilities;

/// Border style for components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Single,
    Double,
    Rounded,
    Heavy,
    Ascii,
}

/// Theme defining colors, spacing, typography for UI components
#[derive(Debug, Clone)]
pub struct Theme {
    pub text_fg: Color,
    pub heading_fg: Color,
    pub label_fg: Color,
    pub error_fg: Color,
    pub success_fg: Color,
    pub warning_fg: Color,
    pub link_fg: Color,

    pub background: Color,
    pub surface: Color,
    pub surface_elevated: Color,

    pub header_title_fg: Color,
    pub header_bg: Option<Color>,

    pub badge_bg: Color,
    pub badge_fg: Color,

    pub status_fg: Color,
    pub status_bg: Option<Color>,

    pub border_color: Color,
    pub focus_border_color: Color,

    pub spacing_xs: u16,
    pub spacing_sm: u16,
    pub spacing_md: u16,
    pub spacing_lg: u16,
    pub spacing_xl: u16,

    pub default_gap: u16,
    pub default_padding: u16,

    pub font_scale: f32,
    pub line_height: f32,
    pub heading_bold: bool,
    pub label_dim: bool,

    pub border_style: BorderStyle,
    pub border_width: u16,

    pub text_direction: TextDirection,
    pub locale: Locale,

    pub accessibility: AccessibilitySettings,

    caps: TerminalCapabilities,
}

impl Theme {
    /// Create a new theme with terminal capabilities
    pub fn new(caps: TerminalCapabilities) -> Self {
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

            caps,
        }
    }

    pub fn header_title_style(&self) -> String {
        format!("{}\x1b[1m", self.header_title_fg.degrade(&self.caps))
    }

    pub fn badge_style(&self) -> String {
        format!(
            "{}{}",
            self.badge_fg.degrade(&self.caps),
            self.badge_bg.bg(&self.caps)
        )
    }

    pub fn status_style(&self) -> String {
        if let Some(bg) = &self.status_bg {
            format!(
                "{}{}",
                self.status_fg.degrade(&self.caps),
                bg.bg(&self.caps)
            )
        } else {
            format!("{}\x1b[7m", self.status_fg.degrade(&self.caps))
        }
    }

    pub fn status_bg_fill(&self) -> String {
        if let Some(bg) = &self.status_bg {
            bg.bg(&self.caps)
        } else {
            "\x1b[7m".to_string()
        }
    }

    pub fn text_style(&self) -> String {
        self.text_fg.degrade(&self.caps)
    }

    pub fn heading_style(&self) -> String {
        let mut style = self.heading_fg.degrade(&self.caps);
        if self.heading_bold {
            style.push_str("\x1b[1m");
        }
        style
    }

    pub fn label_style(&self) -> String {
        let mut style = self.label_fg.degrade(&self.caps);
        if self.label_dim {
            style.push_str("\x1b[2m");
        }
        style
    }

    pub fn error_style(&self) -> String {
        self.error_fg.degrade(&self.caps)
    }

    pub fn success_style(&self) -> String {
        self.success_fg.degrade(&self.caps)
    }

    pub fn warning_style(&self) -> String {
        self.warning_fg.degrade(&self.caps)
    }

    pub fn link_style(&self) -> String {
        format!("{}\x1b[4m", self.link_fg.degrade(&self.caps))
    }

    pub fn background_style(&self) -> String {
        self.background.bg(&self.caps)
    }

    pub fn surface_style(&self) -> String {
        self.surface.bg(&self.caps)
    }

    pub fn surface_elevated_style(&self) -> String {
        self.surface_elevated.bg(&self.caps)
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
    pub horizontal: char,
    pub vertical: char,
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}

impl BorderChars {
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
        let caps = TerminalCapabilities::detect();
        let theme = Theme::new(caps);

        let style = theme.header_title_style();
        assert!(!style.is_empty());
    }
}

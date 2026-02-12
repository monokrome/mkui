//! Color types with automatic degradation support

use crate::terminal::TerminalCapabilities;

/// Color representation with automatic degradation support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// True color RGB
    Rgb(u8, u8, u8),
    /// 256-color palette index
    Palette256(u8),
    /// 16-color ANSI
    Ansi16(AnsiColor),
    /// Basic 8-color
    Basic(BasicColor),
}

/// 16-color ANSI colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

/// Basic 8 colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    /// Create a color from RGB values
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb(r, g, b)
    }

    pub fn white() -> Self {
        Color::Rgb(255, 255, 255)
    }

    pub fn black() -> Self {
        Color::Rgb(0, 0, 0)
    }

    pub fn light_gray() -> Self {
        Color::Rgb(192, 192, 192)
    }

    pub fn dark_gray() -> Self {
        Color::Rgb(128, 128, 128)
    }

    pub fn dark_purple() -> Self {
        Color::Rgb(58, 48, 68)
    }

    /// Degrade color to terminal capabilities
    pub fn degrade(&self, caps: &TerminalCapabilities) -> String {
        if caps.truecolor {
            self.to_truecolor()
        } else if caps.colors_256 {
            self.to_256color()
        } else {
            self.to_ansi16()
        }
    }

    fn to_truecolor(self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    fn to_256color(self) -> String {
        let idx = match self {
            Color::Palette256(idx) => idx,
            _ => {
                let (r, g, b) = self.to_rgb();
                rgb_to_256(r, g, b)
            }
        };
        format!("\x1b[38;5;{}m", idx)
    }

    fn to_ansi16(self) -> String {
        let ansi = match self {
            Color::Ansi16(a) => a,
            Color::Basic(b) => match b {
                BasicColor::Black => AnsiColor::Black,
                BasicColor::Red => AnsiColor::Red,
                BasicColor::Green => AnsiColor::Green,
                BasicColor::Yellow => AnsiColor::Yellow,
                BasicColor::Blue => AnsiColor::Blue,
                BasicColor::Magenta => AnsiColor::Magenta,
                BasicColor::Cyan => AnsiColor::Cyan,
                BasicColor::White => AnsiColor::White,
            },
            _ => {
                let (r, g, b) = self.to_rgb();
                rgb_to_ansi16(r, g, b)
            }
        };

        ansi.to_ansi_code()
    }

    fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Palette256(idx) => palette256_to_rgb(idx),
            Color::Ansi16(a) => a.to_rgb(),
            Color::Basic(b) => b.to_rgb(),
        }
    }

    /// Get background version of this color
    pub fn bg(&self, caps: &TerminalCapabilities) -> String {
        if caps.truecolor {
            let (r, g, b) = self.to_rgb();
            format!("\x1b[48;2;{};{};{}m", r, g, b)
        } else if caps.colors_256 {
            let (r, g, b) = self.to_rgb();
            let idx = rgb_to_256(r, g, b);
            format!("\x1b[48;5;{}m", idx)
        } else {
            let (r, g, b) = self.to_rgb();
            let ansi = rgb_to_ansi16(r, g, b);
            ansi.to_ansi_bg_code()
        }
    }
}

impl AnsiColor {
    pub(crate) fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            AnsiColor::Black => (0, 0, 0),
            AnsiColor::Red => (170, 0, 0),
            AnsiColor::Green => (0, 170, 0),
            AnsiColor::Yellow => (170, 85, 0),
            AnsiColor::Blue => (0, 0, 170),
            AnsiColor::Magenta => (170, 0, 170),
            AnsiColor::Cyan => (0, 170, 170),
            AnsiColor::White => (170, 170, 170),
            AnsiColor::BrightBlack => (85, 85, 85),
            AnsiColor::BrightRed => (255, 85, 85),
            AnsiColor::BrightGreen => (85, 255, 85),
            AnsiColor::BrightYellow => (255, 255, 85),
            AnsiColor::BrightBlue => (85, 85, 255),
            AnsiColor::BrightMagenta => (255, 85, 255),
            AnsiColor::BrightCyan => (85, 255, 255),
            AnsiColor::BrightWhite => (255, 255, 255),
        }
    }

    pub(crate) fn to_ansi_code(self) -> String {
        let code = match self {
            AnsiColor::Black => 30,
            AnsiColor::Red => 31,
            AnsiColor::Green => 32,
            AnsiColor::Yellow => 33,
            AnsiColor::Blue => 34,
            AnsiColor::Magenta => 35,
            AnsiColor::Cyan => 36,
            AnsiColor::White => 37,
            AnsiColor::BrightBlack => 90,
            AnsiColor::BrightRed => 91,
            AnsiColor::BrightGreen => 92,
            AnsiColor::BrightYellow => 93,
            AnsiColor::BrightBlue => 94,
            AnsiColor::BrightMagenta => 95,
            AnsiColor::BrightCyan => 96,
            AnsiColor::BrightWhite => 97,
        };
        format!("\x1b[{}m", code)
    }

    pub(crate) fn to_ansi_bg_code(self) -> String {
        let code = match self {
            AnsiColor::Black => 40,
            AnsiColor::Red => 41,
            AnsiColor::Green => 42,
            AnsiColor::Yellow => 43,
            AnsiColor::Blue => 44,
            AnsiColor::Magenta => 45,
            AnsiColor::Cyan => 46,
            AnsiColor::White => 47,
            AnsiColor::BrightBlack => 100,
            AnsiColor::BrightRed => 101,
            AnsiColor::BrightGreen => 102,
            AnsiColor::BrightYellow => 103,
            AnsiColor::BrightBlue => 104,
            AnsiColor::BrightMagenta => 105,
            AnsiColor::BrightCyan => 106,
            AnsiColor::BrightWhite => 107,
        };
        format!("\x1b[{}m", code)
    }

    pub(crate) fn from_index(idx: u8) -> Self {
        match idx {
            0 => AnsiColor::Black,
            1 => AnsiColor::Red,
            2 => AnsiColor::Green,
            3 => AnsiColor::Yellow,
            4 => AnsiColor::Blue,
            5 => AnsiColor::Magenta,
            6 => AnsiColor::Cyan,
            7 => AnsiColor::White,
            8 => AnsiColor::BrightBlack,
            9 => AnsiColor::BrightRed,
            10 => AnsiColor::BrightGreen,
            11 => AnsiColor::BrightYellow,
            12 => AnsiColor::BrightBlue,
            13 => AnsiColor::BrightMagenta,
            14 => AnsiColor::BrightCyan,
            _ => AnsiColor::BrightWhite,
        }
    }
}

impl BasicColor {
    pub(crate) fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            BasicColor::Black => (0, 0, 0),
            BasicColor::Red => (170, 0, 0),
            BasicColor::Green => (0, 170, 0),
            BasicColor::Yellow => (170, 85, 0),
            BasicColor::Blue => (0, 0, 170),
            BasicColor::Magenta => (170, 0, 170),
            BasicColor::Cyan => (0, 170, 170),
            BasicColor::White => (170, 170, 170),
        }
    }
}

/// Convert RGB to 256-color palette index
pub(crate) fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return ((r - 8) / 10) + 232;
    }

    let r_idx = (r as u16 * 5 / 255) as u8;
    let g_idx = (g as u16 * 5 / 255) as u8;
    let b_idx = (b as u16 * 5 / 255) as u8;

    16 + 36 * r_idx + 6 * g_idx + b_idx
}

/// Convert 256-color palette index to RGB
pub(crate) fn palette256_to_rgb(idx: u8) -> (u8, u8, u8) {
    if idx < 16 {
        AnsiColor::from_index(idx).to_rgb()
    } else if idx >= 232 {
        let gray = 8 + (idx - 232) * 10;
        (gray, gray, gray)
    } else {
        let idx = idx - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        (r, g, b)
    }
}

/// Convert RGB to closest ANSI 16 color
pub(crate) fn rgb_to_ansi16(r: u8, g: u8, b: u8) -> AnsiColor {
    let brightness = (r as u32 + g as u32 + b as u32) / 3;

    if brightness < 32 {
        return AnsiColor::Black;
    }

    if brightness > 128 {
        bright_ansi_color(r, g, b)
    } else {
        dark_ansi_color(r, g, b)
    }
}

fn bright_ansi_color(r: u8, g: u8, b: u8) -> AnsiColor {
    if r > 200 && g > 200 && b > 200 {
        return AnsiColor::BrightWhite;
    }

    match dominant_channel(r, g, b) {
        Some(DominantChannel::Red) => AnsiColor::BrightRed,
        Some(DominantChannel::Green) => AnsiColor::BrightGreen,
        Some(DominantChannel::Blue) => AnsiColor::BrightBlue,
        None if r > 150 && g > 150 => AnsiColor::BrightYellow,
        None if r > 150 && b > 150 => AnsiColor::BrightMagenta,
        None if g > 150 && b > 150 => AnsiColor::BrightCyan,
        None => AnsiColor::White,
    }
}

fn dark_ansi_color(r: u8, g: u8, b: u8) -> AnsiColor {
    match dominant_channel(r, g, b) {
        Some(DominantChannel::Red) => AnsiColor::Red,
        Some(DominantChannel::Green) => AnsiColor::Green,
        Some(DominantChannel::Blue) => AnsiColor::Blue,
        None if r > 100 && g > 100 => AnsiColor::Yellow,
        None if r > 100 && b > 100 => AnsiColor::Magenta,
        None if g > 100 && b > 100 => AnsiColor::Cyan,
        None => AnsiColor::BrightBlack,
    }
}

enum DominantChannel {
    Red,
    Green,
    Blue,
}

fn dominant_channel(r: u8, g: u8, b: u8) -> Option<DominantChannel> {
    if r > g && r > b {
        Some(DominantChannel::Red)
    } else if g > r && g > b {
        Some(DominantChannel::Green)
    } else if b > r && b > g {
        Some(DominantChannel::Blue)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_256() {
        assert_eq!(rgb_to_256(255, 255, 255), 231);
        assert_eq!(rgb_to_256(0, 0, 0), 16);

        let gray_idx = rgb_to_256(128, 128, 128);
        assert!((232..=255).contains(&gray_idx));
    }

    #[test]
    fn test_color_degradation() {
        let caps = TerminalCapabilities {
            kitty_graphics: false,
            sixel: false,
            truecolor: true,
            colors_256: true,
            in_multiplexer: false,
            mouse: true,
        };

        let white = Color::white();
        let code = white.degrade(&caps);
        assert!(code.contains("38;2;255;255;255"));
    }
}

//! Terminal abstraction - geometry, capabilities, and context

use anyhow::{Context, Result};
use std::process::Command;

/// Tmux pane position information
#[derive(Debug, Clone, Copy, Default)]
pub struct TmuxPaneInfo {
    /// Top row of the pane (0-indexed from terminal top)
    pub top: u16,
    /// Left column of the pane (0-indexed from terminal left)
    pub left: u16,
    /// Width of the pane in columns
    pub width: u16,
    /// Height of the pane in rows
    pub height: u16,
}

impl TmuxPaneInfo {
    /// Query tmux for current pane position and size
    pub fn query() -> Option<Self> {
        let output = Command::new("tmux")
            .args([
                "display-message",
                "-p",
                "#{pane_top} #{pane_left} #{pane_width} #{pane_height}",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();

        if parts.len() != 4 {
            return None;
        }

        Some(TmuxPaneInfo {
            top: parts[0].parse().ok()?,
            left: parts[1].parse().ok()?,
            width: parts[2].parse().ok()?,
            height: parts[3].parse().ok()?,
        })
    }
}

/// Terminal geometry and sizing information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalGeometry {
    /// Terminal width in columns (characters)
    pub cols: u16,
    /// Terminal height in rows (lines)
    pub rows: u16,
    /// Estimated width in pixels (if detectable)
    pub pixel_width: Option<u32>,
    /// Estimated height in pixels (if detectable)
    pub pixel_height: Option<u32>,
    /// Estimated character width in pixels
    pub char_width: u16,
    /// Estimated character height in pixels
    pub char_height: u16,
}

impl TerminalGeometry {
    /// Get current terminal geometry
    pub fn detect() -> Result<Self> {
        // Get character dimensions using crossterm
        let (cols, rows) = crossterm::terminal::size().context("Failed to get terminal size")?;

        // Estimate pixel dimensions
        // TODO: Query actual terminal for precise values via escape sequences
        let char_width = 10; // Typical monospace font width
        let char_height = 20; // Typical monospace font height

        let pixel_width = Some(cols as u32 * char_width as u32);
        let pixel_height = Some(rows as u32 * char_height as u32);

        Ok(TerminalGeometry {
            cols,
            rows,
            pixel_width,
            pixel_height,
            char_width,
            char_height,
        })
    }

    /// Get geometry with custom pixel estimates
    pub fn with_char_size(cols: u16, rows: u16, char_width: u16, char_height: u16) -> Self {
        let pixel_width = Some(cols as u32 * char_width as u32);
        let pixel_height = Some(rows as u32 * char_height as u32);

        TerminalGeometry {
            cols,
            rows,
            pixel_width,
            pixel_height,
            char_width,
            char_height,
        }
    }
}

/// Terminal capability detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    /// Supports Kitty graphics protocol
    pub kitty_graphics: bool,
    /// Supports Sixel graphics
    pub sixel: bool,
    /// Supports 24-bit true color
    pub truecolor: bool,
    /// Supports 256 colors
    pub colors_256: bool,
    /// Inside tmux/screen multiplexer
    pub in_multiplexer: bool,
    /// Supports mouse events
    pub mouse: bool,
}

impl TerminalCapabilities {
    /// Detect terminal capabilities
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        let tmux = std::env::var("TMUX").is_ok();
        let kitty_window = std::env::var("KITTY_WINDOW_ID").is_ok();

        // Detect Kitty graphics
        let kitty_graphics = kitty_window || term.contains("kitty");

        // Detect Sixel support
        let sixel = term.contains("mlterm") || term.contains("xterm");

        // Detect truecolor
        let truecolor =
            colorterm.contains("truecolor") || colorterm.contains("24bit") || kitty_window;

        // 256 color support is pretty universal now
        let colors_256 = term.contains("256") || truecolor;

        // Mouse support via crossterm
        let mouse = true; // Most modern terminals support this

        TerminalCapabilities {
            kitty_graphics,
            sixel,
            truecolor,
            colors_256,
            in_multiplexer: tmux,
            mouse,
        }
    }

    /// Check if we need tmux passthrough for Kitty graphics
    pub fn needs_kitty_passthrough(&self) -> bool {
        self.kitty_graphics && self.in_multiplexer
    }
}

/// Complete terminal context combining geometry and capabilities
#[derive(Debug, Clone)]
pub struct TerminalContext {
    pub geometry: TerminalGeometry,
    pub capabilities: TerminalCapabilities,
}

impl TerminalContext {
    /// Create a new terminal context by detecting current environment
    pub fn detect() -> Result<Self> {
        Ok(TerminalContext {
            geometry: TerminalGeometry::detect()?,
            capabilities: TerminalCapabilities::detect(),
        })
    }

    /// Refresh geometry (e.g., after terminal resize)
    pub fn refresh_geometry(&mut self) -> Result<()> {
        self.geometry = TerminalGeometry::detect()?;
        Ok(())
    }

    /// Get pixel dimensions if available
    pub fn pixel_dimensions(&self) -> Option<(u32, u32)> {
        match (self.geometry.pixel_width, self.geometry.pixel_height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    /// Get character dimensions
    pub fn char_dimensions(&self) -> (u16, u16) {
        (self.geometry.cols, self.geometry.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_with_char_size() {
        let geom = TerminalGeometry::with_char_size(80, 24, 10, 20);
        assert_eq!(geom.cols, 80);
        assert_eq!(geom.rows, 24);
        assert_eq!(geom.pixel_width, Some(800));
        assert_eq!(geom.pixel_height, Some(480));
    }

    #[test]
    fn test_capabilities_detect() {
        let caps = TerminalCapabilities::detect();
        // Should always detect something reasonable
        assert!(caps.colors_256 || !caps.truecolor);
    }
}

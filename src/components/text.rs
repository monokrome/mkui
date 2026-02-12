//! Text component for displaying styled text

use crate::component::Component;
use crate::context::{RenderContext, UseTheme};
use crate::event::EventHandler;
use crate::i18n::TextDirection;
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Text alignment - supports both logical and physical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Align to the start (left in LTR, right in RTL)
    Start,
    /// Align to the end (right in LTR, left in RTL)
    End,
    /// Center alignment (always centered)
    Center,
    /// Force left alignment (ignores text direction)
    ForceLeft,
    /// Force right alignment (ignores text direction)
    ForceRight,
}

/// Physical alignment (after resolving logical alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// Resolve logical alignment to physical alignment based on text direction
    pub fn resolve(&self, direction: TextDirection) -> PhysicalAlign {
        match (self, direction) {
            (TextAlign::Start, TextDirection::LeftToRight) => PhysicalAlign::Left,
            (TextAlign::Start, TextDirection::RightToLeft) => PhysicalAlign::Right,
            (TextAlign::End, TextDirection::LeftToRight) => PhysicalAlign::Right,
            (TextAlign::End, TextDirection::RightToLeft) => PhysicalAlign::Left,
            (TextAlign::Center, _) => PhysicalAlign::Center,
            (TextAlign::ForceLeft, _) => PhysicalAlign::Left,
            (TextAlign::ForceRight, _) => PhysicalAlign::Right,
        }
    }
}

/// Text component
pub struct Text {
    pub(crate) content: String,
    pub(crate) style: String,
    pub(crate) align: TextAlign,
    pub(crate) dirty: bool,
}

impl Text {
    /// Create new text component
    pub fn new(content: impl Into<String>) -> Self {
        Text {
            content: content.into(),
            style: String::new(),
            align: TextAlign::Start,
            dirty: true,
        }
    }

    /// Set text style (ANSI codes)
    pub fn with_style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self.dirty = true;
        self
    }

    /// Set text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self.dirty = true;
        self
    }

    /// Update text content
    pub fn set_text(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.dirty = true;
    }

    /// Get text content
    pub fn text(&self) -> &str {
        &self.content
    }
}

impl EventHandler for Text {}

impl Component for Text {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        if self.content.is_empty() {
            return Ok(());
        }

        // Resolve logical alignment to physical based on text direction
        let text_direction = self.use_text_direction(ctx);
        let physical_align = self.align.resolve(text_direction);

        // Calculate x position based on resolved physical alignment
        let text_len = self.content.len() as u16;
        let x = match physical_align {
            PhysicalAlign::Left => bounds.x,
            PhysicalAlign::Center => {
                let offset = (bounds.width.saturating_sub(text_len)) / 2;
                bounds.x.saturating_add(offset)
            }
            PhysicalAlign::Right => {
                let offset = bounds.width.saturating_sub(text_len);
                bounds.x.saturating_add(offset)
            }
        };

        // Render text at calculated position
        renderer.move_cursor(x, bounds.y)?;

        if self.style.is_empty() {
            renderer.write_text(&self.content)?;
        } else {
            renderer.write_styled(&self.content, &self.style)?;
        }

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (self.content.len() as u16, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "Text"
    }
}

/// Common ANSI style constants
pub mod styles {
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const ITALIC: &str = "\x1b[3m";
    pub const UNDERLINE: &str = "\x1b[4m";

    pub const BLACK: &str = "\x1b[30m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_WHITE: &str = "\x1b[47m";
    pub const BG_BLUE: &str = "\x1b[44m";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::TextDirection;

    #[test]
    fn test_text_align_resolve_ltr() {
        // In LTR, Start = Left, End = Right
        assert_eq!(
            TextAlign::Start.resolve(TextDirection::LeftToRight),
            PhysicalAlign::Left
        );
        assert_eq!(
            TextAlign::End.resolve(TextDirection::LeftToRight),
            PhysicalAlign::Right
        );
        assert_eq!(
            TextAlign::Center.resolve(TextDirection::LeftToRight),
            PhysicalAlign::Center
        );
    }

    #[test]
    fn test_text_align_resolve_rtl() {
        // In RTL, Start = Right, End = Left (reversed!)
        assert_eq!(
            TextAlign::Start.resolve(TextDirection::RightToLeft),
            PhysicalAlign::Right
        );
        assert_eq!(
            TextAlign::End.resolve(TextDirection::RightToLeft),
            PhysicalAlign::Left
        );
        assert_eq!(
            TextAlign::Center.resolve(TextDirection::RightToLeft),
            PhysicalAlign::Center
        );
    }

    #[test]
    fn test_text_align_force_ignores_direction() {
        // Force should ignore text direction
        assert_eq!(
            TextAlign::ForceLeft.resolve(TextDirection::RightToLeft),
            PhysicalAlign::Left
        );
        assert_eq!(
            TextAlign::ForceRight.resolve(TextDirection::LeftToRight),
            PhysicalAlign::Right
        );
    }
}

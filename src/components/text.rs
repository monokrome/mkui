//! Text component for displaying styled text

use crate::component::Component;
use crate::context::{RenderContext, UseTheme};
use crate::event::EventHandler;
use crate::i18n::TextDirection;
use crate::layout::Rect;
use crate::render::Renderer;
use crate::signal::Signal;
use crate::style::Style;
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
    /// Align to the left edge
    Left,
    /// Align to the center
    Center,
    /// Align to the right edge
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
    /// The text content to display
    pub(crate) content: Signal<String>,
    /// Visual style applied to the text
    pub(crate) style: Signal<Style>,
    /// Text alignment mode
    pub(crate) align: TextAlign,
}

impl Text {
    /// Create new text component
    pub fn new(content: impl Into<String>) -> Self {
        Text {
            content: Signal::new(content.into()),
            style: Signal::new(Style::new()),
            align: TextAlign::Start,
        }
    }

    /// Set text style
    pub fn with_style(mut self, style: Style) -> Self {
        self.style.set(style);
        self
    }

    /// Set text alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Update text content
    pub fn set_text(&mut self, content: impl Into<String>) {
        self.content.set(content.into());
    }

    /// Get text content
    pub fn text(&self) -> &str {
        self.content.get()
    }
}

impl EventHandler for Text {}

impl Component for Text {
    fn render(&mut self, renderer: &mut dyn Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        let content = self.content.get();
        if content.is_empty() {
            return Ok(());
        }

        let text_direction = self.use_text_direction(ctx);
        let physical_align = self.align.resolve(text_direction);

        let text_len = content.len() as u16;
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

        renderer.move_cursor(x, bounds.y)?;

        let style = self.style.get();
        if style.is_empty() {
            renderer.write_text(content)?;
        } else {
            renderer.write_styled(content, style)?;
        }

        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (self.content.get().len() as u16, 1)
    }

    fn generation(&self) -> u64 {
        self.content.generation() + self.style.generation()
    }

    fn name(&self) -> &str {
        "Text"
    }
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

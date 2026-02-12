//! Popup/Modal component for overlays
//!
//! Provides modal dialogs with:
//! - Centered positioning
//! - Border/chrome styling
//! - Focus trapping
//! - ESC to close

mod confirm;

pub use confirm::ConfirmPopup;

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Popup position on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupPosition {
    #[default]
    Center,
    Top,
    Bottom,
    Fixed {
        x: u16,
        y: u16,
    },
}

/// Popup border style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupBorderStyle {
    None,
    #[default]
    Single,
    Double,
    Rounded,
}

impl PopupBorderStyle {
    fn chars(&self) -> Option<BorderChars> {
        match self {
            PopupBorderStyle::None => None,
            PopupBorderStyle::Single => Some(BorderChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
            }),
            PopupBorderStyle::Double => Some(BorderChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
            }),
            PopupBorderStyle::Rounded => Some(BorderChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BorderChars {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
}

/// Result from popup interaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupResult {
    Open,
    Cancelled,
    Confirmed,
    Custom(String),
}

/// Modal popup component
pub struct Popup {
    content: Box<dyn Component>,
    title: Option<String>,
    visible: bool,
    size: Option<(u16, u16)>,
    position: PopupPosition,
    border_style: PopupBorderStyle,
    close_on_escape: bool,
    trap_focus: bool,
    result: PopupResult,
    dirty: bool,
}

impl std::fmt::Debug for Popup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup")
            .field("title", &self.title)
            .field("visible", &self.visible)
            .field("size", &self.size)
            .field("position", &self.position)
            .field("border_style", &self.border_style)
            .field("result", &self.result)
            .finish()
    }
}

impl Popup {
    pub fn new(content: Box<dyn Component>) -> Self {
        Self {
            content,
            title: None,
            visible: false,
            size: None,
            position: PopupPosition::Center,
            border_style: PopupBorderStyle::Single,
            close_on_escape: true,
            trap_focus: true,
            result: PopupResult::Open,
            dirty: true,
        }
    }

    pub fn message(text: impl Into<String>) -> Self {
        Self::new(Box::new(MessageContent { text: text.into() }))
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.size = Some((width, height));
        self
    }

    pub fn with_position(mut self, position: PopupPosition) -> Self {
        self.position = position;
        self
    }

    pub fn with_border(mut self, style: PopupBorderStyle) -> Self {
        self.border_style = style;
        self
    }

    pub fn with_close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    pub fn with_trap_focus(mut self, trap: bool) -> Self {
        self.trap_focus = trap;
        self
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.result = PopupResult::Open;
        self.dirty = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.dirty = true;
    }

    pub fn cancel(&mut self) {
        self.result = PopupResult::Cancelled;
        self.close();
    }

    pub fn confirm(&mut self) {
        self.result = PopupResult::Confirmed;
        self.close();
    }

    pub fn close_with(&mut self, action: impl Into<String>) {
        self.result = PopupResult::Custom(action.into());
        self.close();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn result(&self) -> &PopupResult {
        &self.result
    }

    pub fn take_result(&mut self) -> PopupResult {
        std::mem::replace(&mut self.result, PopupResult::Open)
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
        self.dirty = true;
    }

    pub fn content_mut(&mut self) -> &mut Box<dyn Component> {
        &mut self.content
    }

    fn calculate_bounds(&self, parent: Rect) -> Rect {
        let (width, height) = self.size.unwrap_or_else(|| {
            let (min_w, min_h) = self.content.min_size();
            let border_add = if self.border_style == PopupBorderStyle::None {
                0
            } else {
                2
            };
            (
                (min_w + border_add).min(parent.width),
                (min_h + border_add).min(parent.height),
            )
        });

        let (x, y) = match self.position {
            PopupPosition::Center => (
                parent.x + (parent.width.saturating_sub(width)) / 2,
                parent.y + (parent.height.saturating_sub(height)) / 2,
            ),
            PopupPosition::Top => (
                parent.x + (parent.width.saturating_sub(width)) / 2,
                parent.y + 1,
            ),
            PopupPosition::Bottom => (
                parent.x + (parent.width.saturating_sub(width)) / 2,
                parent.y + parent.height.saturating_sub(height + 1),
            ),
            PopupPosition::Fixed { x, y } => (x, y),
        };

        Rect::new(x, y, width, height)
    }

    fn content_bounds(&self, popup_bounds: Rect) -> Rect {
        if self.border_style == PopupBorderStyle::None {
            popup_bounds
        } else {
            Rect::new(
                popup_bounds.x + 1,
                popup_bounds.y + 1,
                popup_bounds.width.saturating_sub(2),
                popup_bounds.height.saturating_sub(2),
            )
        }
    }
}

impl EventHandler for Popup {
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.visible {
            return false;
        }

        match event {
            Event::Key(Key::Esc) if self.close_on_escape => {
                self.cancel();
                return true;
            }
            Event::Key(Key::Enter) => {
                self.confirm();
                return true;
            }
            _ => {}
        }

        if self.content.handle_event(event) {
            return true;
        }

        self.trap_focus
    }
}

impl Component for Popup {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        let popup_bounds = self.calculate_bounds(bounds);
        let content_bounds = self.content_bounds(popup_bounds);

        if let Some(chars) = self.border_style.chars() {
            renderer.move_cursor(popup_bounds.x, popup_bounds.y)?;
            renderer.write_text(&chars.top_left.to_string())?;

            if let Some(title) = &self.title {
                let title_space = (popup_bounds.width as usize).saturating_sub(4);
                let display_title = if title.len() > title_space {
                    format!(" {}... ", &title[..title_space.saturating_sub(3)])
                } else {
                    format!(" {} ", title)
                };

                let padding_left = (popup_bounds.width as usize - display_title.len() - 2) / 2;
                let padding_right =
                    popup_bounds.width as usize - display_title.len() - 2 - padding_left;

                for _ in 0..padding_left {
                    renderer.write_text(&chars.horizontal.to_string())?;
                }
                renderer.write_text(&display_title)?;
                for _ in 0..padding_right {
                    renderer.write_text(&chars.horizontal.to_string())?;
                }
            } else {
                for _ in 0..(popup_bounds.width - 2) {
                    renderer.write_text(&chars.horizontal.to_string())?;
                }
            }
            renderer.write_text(&chars.top_right.to_string())?;

            for y in 1..(popup_bounds.height - 1) {
                renderer.move_cursor(popup_bounds.x, popup_bounds.y + y)?;
                renderer.write_text(&chars.vertical.to_string())?;

                for _ in 0..(popup_bounds.width - 2) {
                    renderer.write_text(" ")?;
                }

                renderer.write_text(&chars.vertical.to_string())?;
            }

            renderer.move_cursor(popup_bounds.x, popup_bounds.y + popup_bounds.height - 1)?;
            renderer.write_text(&chars.bottom_left.to_string())?;
            for _ in 0..(popup_bounds.width - 2) {
                renderer.write_text(&chars.horizontal.to_string())?;
            }
            renderer.write_text(&chars.bottom_right.to_string())?;
        }

        self.content.render(renderer, content_bounds, ctx)?;

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        if let Some((w, h)) = self.size {
            (w, h)
        } else {
            let (min_w, min_h) = self.content.min_size();
            let border_add = if self.border_style == PopupBorderStyle::None {
                0
            } else {
                2
            };
            (min_w + border_add, min_h + border_add)
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.content.mark_dirty();
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.content.is_dirty()
    }

    fn name(&self) -> &str {
        "Popup"
    }
}

struct MessageContent {
    text: String,
}

impl EventHandler for MessageContent {}

impl Component for MessageContent {
    fn render(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        let lines: Vec<&str> = self.text.lines().collect();

        for (i, line) in lines.iter().enumerate().take(bounds.height as usize) {
            renderer.move_cursor(bounds.x, bounds.y + i as u16)?;
            let display = if line.len() > bounds.width as usize {
                &line[..bounds.width as usize]
            } else {
                line
            };
            renderer.write_text(display)?;
        }

        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        let lines: Vec<&str> = self.text.lines().collect();
        let max_width = lines.iter().map(|l| l.len()).max().unwrap_or(10) as u16;
        let height = lines.len().max(1) as u16;
        (max_width, height)
    }

    fn name(&self) -> &str {
        "MessageContent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContent;

    impl EventHandler for TestContent {}

    impl Component for TestContent {
        fn render(
            &mut self,
            _renderer: &mut Renderer,
            _bounds: Rect,
            _ctx: &RenderContext,
        ) -> Result<()> {
            Ok(())
        }

        fn min_size(&self) -> (u16, u16) {
            (20, 10)
        }

        fn name(&self) -> &str {
            "TestContent"
        }
    }

    #[test]
    fn test_popup_visibility() {
        let mut popup = Popup::new(Box::new(TestContent));

        assert!(!popup.is_visible());

        popup.show();
        assert!(popup.is_visible());

        popup.close();
        assert!(!popup.is_visible());
    }

    #[test]
    fn test_popup_results() {
        let mut popup = Popup::new(Box::new(TestContent));
        popup.show();

        popup.confirm();
        assert_eq!(popup.result(), &PopupResult::Confirmed);
        assert!(!popup.is_visible());

        popup.show();
        popup.cancel();
        assert_eq!(popup.result(), &PopupResult::Cancelled);

        popup.show();
        popup.close_with("custom_action");
        assert_eq!(
            popup.result(),
            &PopupResult::Custom("custom_action".to_string())
        );
    }

    #[test]
    fn test_bounds_calculation() {
        let popup = Popup::new(Box::new(TestContent)).with_size(40, 20);

        let parent = Rect::new(0, 0, 80, 24);
        let bounds = popup.calculate_bounds(parent);

        assert_eq!(bounds.x, 20);
        assert_eq!(bounds.y, 2);
        assert_eq!(bounds.width, 40);
        assert_eq!(bounds.height, 20);
    }

    #[test]
    fn test_escape_handling() {
        let mut popup = Popup::new(Box::new(TestContent)).with_close_on_escape(true);

        popup.show();

        let handled = popup.handle_event(&Event::Key(Key::Esc));
        assert!(handled);
        assert!(!popup.is_visible());
        assert_eq!(popup.result(), &PopupResult::Cancelled);
    }

    #[test]
    fn test_confirm_popup() {
        let popup = ConfirmPopup::new("Delete file?")
            .with_title("Warning")
            .build();

        assert_eq!(popup.title(), Some("Warning"));
    }
}

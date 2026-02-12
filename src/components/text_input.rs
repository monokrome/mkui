//! Text input component with cursor and editing support
//!
//! Provides a reusable text input with:
//! - Cursor positioning and movement
//! - Basic editing (insert, delete, backspace)
//! - Navigation (home, end, left, right, word jumps)
//! - Submission handling (enter key)
//! - Optional prompt prefix

use crate::component::Component;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Text input submission callback type
pub type OnSubmit = Box<dyn FnMut(&str)>;

/// Text input component
pub struct TextInput {
    /// Input buffer
    buffer: String,
    /// Cursor position (byte offset)
    cursor: usize,
    /// Prompt text displayed before input
    prompt: String,
    /// Style for the prompt (ANSI codes)
    prompt_style: String,
    /// Style for the input text (ANSI codes)
    input_style: String,
    /// Style for cursor (ANSI codes)
    cursor_style: String,
    /// Whether this input is focused
    focused: bool,
    /// Component dirty flag
    dirty: bool,
    /// Callback when Enter is pressed
    on_submit: Option<OnSubmit>,
}

impl TextInput {
    /// Create a new text input with the given prompt
    pub fn new(prompt: &str) -> Self {
        TextInput {
            buffer: String::new(),
            cursor: 0,
            prompt: prompt.to_string(),
            prompt_style: String::new(),
            input_style: String::new(),
            cursor_style: "\x1b[7m".to_string(), // Inverse video by default
            focused: false,
            dirty: true,
            on_submit: None,
        }
    }

    /// Set the prompt style
    pub fn with_prompt_style(mut self, style: impl Into<String>) -> Self {
        self.prompt_style = style.into();
        self
    }

    /// Set the input text style
    pub fn with_input_style(mut self, style: impl Into<String>) -> Self {
        self.input_style = style.into();
        self
    }

    /// Set the cursor style
    pub fn with_cursor_style(mut self, style: impl Into<String>) -> Self {
        self.cursor_style = style.into();
        self
    }

    /// Set submission callback
    pub fn on_submit<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&str) + 'static,
    {
        self.on_submit = Some(Box::new(callback));
        self
    }

    /// Get current input value
    pub fn value(&self) -> &str {
        &self.buffer
    }

    /// Set the input value
    pub fn set_value(&mut self, value: &str) {
        self.buffer = value.to_string();
        self.cursor = self.buffer.len();
        self.dirty = true;
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.dirty = true;
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// Check if input is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Insert character at cursor
    fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.dirty = true;
    }

    /// Delete character before cursor (backspace)
    fn delete_char_before(&mut self) {
        if self.cursor > 0 {
            // Find the previous character boundary
            let prev_boundary = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.buffer.remove(prev_boundary);
            self.cursor = prev_boundary;
            self.dirty = true;
        }
    }

    /// Delete character at cursor (delete key)
    fn delete_char_at(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.remove(self.cursor);
            self.dirty = true;
        }
    }

    /// Move cursor left
    fn move_left(&mut self) {
        if self.cursor > 0 {
            // Find previous character boundary
            self.cursor = self.buffer[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.dirty = true;
        }
    }

    /// Move cursor right
    fn move_right(&mut self) {
        if self.cursor < self.buffer.len() {
            // Find next character boundary
            self.cursor = self.buffer[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.buffer.len());
            self.dirty = true;
        }
    }

    /// Move cursor to start
    fn move_to_start(&mut self) {
        if self.cursor != 0 {
            self.cursor = 0;
            self.dirty = true;
        }
    }

    /// Move cursor to end
    fn move_to_end(&mut self) {
        if self.cursor != self.buffer.len() {
            self.cursor = self.buffer.len();
            self.dirty = true;
        }
    }

    /// Move cursor to previous word boundary
    fn move_word_left(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let chars: Vec<(usize, char)> = self.buffer[..self.cursor].char_indices().collect();
        if chars.is_empty() {
            return;
        }

        let mut i = chars.len() - 1;

        // Skip whitespace
        while i > 0 && chars[i].1.is_whitespace() {
            i -= 1;
        }

        // Skip word characters
        while i > 0 && !chars[i - 1].1.is_whitespace() {
            i -= 1;
        }

        self.cursor = chars.get(i).map(|(idx, _)| *idx).unwrap_or(0);
        self.dirty = true;
    }

    /// Move cursor to next word boundary
    fn move_word_right(&mut self) {
        if self.cursor >= self.buffer.len() {
            return;
        }

        let chars: Vec<(usize, char)> = self.buffer[self.cursor..].char_indices().collect();
        if chars.is_empty() {
            return;
        }

        let mut i = 0;

        // Skip current word characters
        while i < chars.len() && !chars[i].1.is_whitespace() {
            i += 1;
        }

        // Skip whitespace
        while i < chars.len() && chars[i].1.is_whitespace() {
            i += 1;
        }

        self.cursor = if i < chars.len() {
            self.cursor + chars[i].0
        } else {
            self.buffer.len()
        };
        self.dirty = true;
    }

    /// Delete word before cursor (Ctrl+W)
    fn delete_word_before(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let original_cursor = self.cursor;
        self.move_word_left();
        let new_cursor = self.cursor;

        // Delete from new position to original position
        self.buffer.drain(new_cursor..original_cursor);
        self.dirty = true;
    }

    /// Delete from cursor to end of line (Ctrl+K)
    fn delete_to_end(&mut self) {
        if self.cursor < self.buffer.len() {
            self.buffer.truncate(self.cursor);
            self.dirty = true;
        }
    }

    /// Delete from cursor to start of line (Ctrl+U)
    fn delete_to_start(&mut self) {
        if self.cursor > 0 {
            self.buffer.drain(..self.cursor);
            self.cursor = 0;
            self.dirty = true;
        }
    }

    /// Handle paste event
    fn handle_paste(&mut self, text: &str) {
        // Only insert single-line content (strip newlines)
        let clean_text: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        self.buffer.insert_str(self.cursor, &clean_text);
        self.cursor += clean_text.len();
        self.dirty = true;
    }

    fn write_input_text(&self, renderer: &mut Renderer, text: &str) -> Result<()> {
        if !self.input_style.is_empty() {
            renderer.write_styled(text, &self.input_style)
        } else {
            renderer.write_text(text)
        }
    }

    fn handle_key(&mut self, key: &Key) -> bool {
        match key {
            Key::Char(c) => {
                self.insert_char(*c);
                true
            }
            Key::Enter => {
                if let Some(callback) = &mut self.on_submit {
                    callback(&self.buffer);
                }
                true
            }
            Key::Esc => false,
            _ => self.handle_editing_key(key) || self.handle_navigation_key(key),
        }
    }

    fn handle_editing_key(&mut self, key: &Key) -> bool {
        match key {
            Key::Backspace => self.delete_char_before(),
            Key::Delete => self.delete_char_at(),
            Key::Ctrl('w') => self.delete_word_before(),
            Key::Ctrl('k') => self.delete_to_end(),
            Key::Ctrl('u') => self.delete_to_start(),
            _ => return false,
        }
        true
    }

    fn handle_navigation_key(&mut self, key: &Key) -> bool {
        match key {
            Key::Left => self.move_left(),
            Key::Right => self.move_right(),
            Key::Home | Key::Ctrl('a') => self.move_to_start(),
            Key::End | Key::Ctrl('e') => self.move_to_end(),
            Key::Alt('b') => self.move_word_left(),
            Key::Alt('f') => self.move_word_right(),
            _ => return false,
        }
        true
    }
}

impl EventHandler for TextInput {
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.focused {
            return false;
        }

        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Paste(text) => {
                self.handle_paste(text);
                true
            }
            _ => false,
        }
    }

    fn on_focus(&mut self) {
        self.focused = true;
        self.dirty = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
        self.dirty = true;
    }
}

impl Component for TextInput {
    fn render(
        &mut self,
        renderer: &mut Renderer,
        bounds: Rect,
        _ctx: &RenderContext,
    ) -> Result<()> {
        renderer.move_cursor(bounds.x, bounds.y)?;

        // Render prompt
        if !self.prompt.is_empty() {
            if self.prompt_style.is_empty() {
                renderer.write_text(&self.prompt)?;
            } else {
                renderer.write_styled(&self.prompt, &self.prompt_style)?;
            }
        }

        // Calculate available width for input
        let prompt_len = self.prompt.chars().count() as u16;
        let available_width = bounds.width.saturating_sub(prompt_len);

        if available_width == 0 {
            self.dirty = false;
            return Ok(());
        }

        // Calculate visible portion of buffer (scroll if needed)
        let cursor_char_pos = self.buffer[..self.cursor].chars().count();
        let _buffer_char_len = self.buffer.chars().count();

        // Determine scroll offset to keep cursor visible
        let scroll_offset = if cursor_char_pos >= available_width as usize {
            cursor_char_pos - (available_width as usize - 1)
        } else {
            0
        };

        // Get visible text
        let visible_chars: String = self
            .buffer
            .chars()
            .skip(scroll_offset)
            .take(available_width as usize)
            .collect();

        let visible_cursor_pos = cursor_char_pos - scroll_offset;

        // Render text with cursor
        if self.focused && visible_cursor_pos < visible_chars.chars().count() {
            let before: String = visible_chars.chars().take(visible_cursor_pos).collect();
            let cursor_char: String = visible_chars
                .chars()
                .nth(visible_cursor_pos)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            let after: String = visible_chars.chars().skip(visible_cursor_pos + 1).collect();

            self.write_input_text(renderer, &before)?;
            renderer.write_styled(&cursor_char, &self.cursor_style)?;
            self.write_input_text(renderer, &after)?;
        } else if self.focused {
            self.write_input_text(renderer, &visible_chars)?;
            renderer.write_styled(" ", &self.cursor_style)?;
        } else {
            self.write_input_text(renderer, &visible_chars)?;
        }

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        // Minimum: prompt + at least some space for input
        let prompt_len = self.prompt.chars().count() as u16;
        (prompt_len + 10, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn name(&self) -> &str {
        "TextInput"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_creation() {
        let input = TextInput::new(": ");
        assert_eq!(input.value(), "");
        assert!(input.is_empty());
    }

    #[test]
    fn test_insert_and_cursor() {
        let mut input = TextInput::new("");
        input.focused = true;

        input.insert_char('h');
        input.insert_char('e');
        input.insert_char('l');
        input.insert_char('l');
        input.insert_char('o');

        assert_eq!(input.value(), "hello");
        assert_eq!(input.cursor_position(), 5);
    }

    #[test]
    fn test_navigation() {
        let mut input = TextInput::new("");
        input.set_value("hello world");

        input.move_to_start();
        assert_eq!(input.cursor_position(), 0);

        input.move_to_end();
        assert_eq!(input.cursor_position(), 11);

        input.move_left();
        assert_eq!(input.cursor_position(), 10);

        input.move_right();
        assert_eq!(input.cursor_position(), 11);
    }

    #[test]
    fn test_deletion() {
        let mut input = TextInput::new("");
        input.set_value("hello");

        input.delete_char_before();
        assert_eq!(input.value(), "hell");

        input.move_to_start();
        input.delete_char_at();
        assert_eq!(input.value(), "ell");
    }

    #[test]
    fn test_word_navigation() {
        let mut input = TextInput::new("");
        input.set_value("hello world test");

        input.move_to_start();
        input.move_word_right();
        // Should be at 'w' in 'world'
        assert_eq!(input.cursor_position(), 6);

        input.move_word_right();
        // Should be at 't' in 'test'
        assert_eq!(input.cursor_position(), 12);

        input.move_word_left();
        // Should be back at 'w' in 'world'
        assert_eq!(input.cursor_position(), 6);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("");
        input.set_value("some text");

        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_position(), 0);
    }
}

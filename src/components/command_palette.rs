//! Command palette component - Vim-style command line
//!
//! Provides a command input interface for:
//! - Ex commands (`:`)
//! - Forward search (`/`)
//! - Backward search (`?`)
//! - Shell commands (`!`)
//!
//! Features:
//! - Command history with navigation
//! - Tab completion support
//! - Prompt indicator based on mode

use crate::component::Component;
use crate::components::text_input::TextInput;
use crate::context::RenderContext;
use crate::event::{Event, EventHandler, Key};
use crate::layout::Rect;
use crate::render::Renderer;
use anyhow::Result;

/// Command mode determines the prompt character and behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandMode {
    /// Ex command mode (`:`)
    Ex,
    /// Forward search (`/`)
    Search,
    /// Backward search (`?`)
    SearchBack,
    /// Shell command (`!`)
    Shell,
}

impl CommandMode {
    /// Get the prompt character for this mode
    pub fn prompt(&self) -> &'static str {
        match self {
            CommandMode::Ex => ":",
            CommandMode::Search => "/",
            CommandMode::SearchBack => "?",
            CommandMode::Shell => "!",
        }
    }

    /// Get the mode name
    pub fn name(&self) -> &'static str {
        match self {
            CommandMode::Ex => "Ex",
            CommandMode::Search => "Search",
            CommandMode::SearchBack => "SearchBack",
            CommandMode::Shell => "Shell",
        }
    }
}

/// Result of command execution
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command executed successfully
    Success(Option<String>),
    /// Command execution failed
    Error(String),
    /// Command not found
    NotFound,
    /// Empty command (no-op)
    Empty,
}

/// Trait for command execution
///
/// Implement this trait to handle commands from the palette.
pub trait CommandExecutor {
    /// Execute a command string
    ///
    /// Returns the result of execution.
    fn execute(&mut self, command: &str, mode: CommandMode) -> CommandResult;

    /// Get completions for a partial command
    ///
    /// Returns a list of possible completions.
    fn complete(&self, partial: &str, mode: CommandMode) -> Vec<String>;
}

/// Command palette component
///
/// A Vim-style command line that sits at the bottom of the screen.
pub struct CommandPalette {
    /// Text input for command entry
    input: TextInput,
    /// Current command mode
    mode: CommandMode,
    /// Command history
    history: Vec<String>,
    /// Current position in history (None = new command)
    history_index: Option<usize>,
    /// Maximum history size
    max_history: usize,
    /// Current completions
    completions: Vec<String>,
    /// Current completion index
    completion_index: Option<usize>,
    /// Whether the palette is active/visible
    active: bool,
    /// Last error message
    last_error: Option<String>,
    /// Last message (success feedback)
    last_message: Option<String>,
    /// Component dirty flag
    dirty: bool,
    /// Saved input before history navigation
    saved_input: Option<String>,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new() -> Self {
        CommandPalette {
            input: TextInput::new(":"),
            mode: CommandMode::Ex,
            history: Vec::new(),
            history_index: None,
            max_history: 100,
            completions: Vec::new(),
            completion_index: None,
            active: false,
            last_error: None,
            last_message: None,
            dirty: true,
            saved_input: None,
        }
    }

    /// Activate the command palette with the given mode
    pub fn activate(&mut self, mode: CommandMode) {
        self.mode = mode;
        self.input = TextInput::new(mode.prompt());
        self.input.on_focus();
        self.active = true;
        self.history_index = None;
        self.completions.clear();
        self.completion_index = None;
        self.last_error = None;
        self.saved_input = None;
        self.dirty = true;
    }

    /// Deactivate the command palette
    pub fn deactivate(&mut self) {
        self.active = false;
        self.input.on_blur();
        self.input.clear();
        self.completions.clear();
        self.completion_index = None;
        self.saved_input = None;
        self.dirty = true;
    }

    /// Check if the palette is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get current command mode
    pub fn mode(&self) -> CommandMode {
        self.mode
    }

    /// Get current input value
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Get last error message
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Get last success message
    pub fn last_message(&self) -> Option<&str> {
        self.last_message.as_deref()
    }

    /// Clear last error
    pub fn clear_error(&mut self) {
        self.last_error = None;
        self.dirty = true;
    }

    /// Clear last message
    pub fn clear_message(&mut self) {
        self.last_message = None;
        self.dirty = true;
    }

    /// Set error message
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.last_error = Some(error.into());
        self.dirty = true;
    }

    /// Set success message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.last_message = Some(message.into());
        self.dirty = true;
    }

    /// Execute the current command
    ///
    /// Returns the command string that should be executed.
    /// The actual execution is handled by the parent component.
    pub fn submit(&mut self) -> Option<String> {
        let command = self.input.value().to_string();

        if command.is_empty() {
            self.deactivate();
            return None;
        }

        // Add to history if different from last entry
        if self.history.last().map(|s| s.as_str()) != Some(&command) {
            self.history.push(command.clone());
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }

        self.deactivate();
        Some(command)
    }

    /// Cancel input and deactivate
    pub fn cancel(&mut self) {
        self.deactivate();
    }

    /// Navigate history up (older)
    fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // Save current input when starting history navigation
        if self.history_index.is_none() {
            self.saved_input = Some(self.input.value().to_string());
        }

        match self.history_index {
            None => {
                self.history_index = Some(self.history.len() - 1);
                self.input.set_value(&self.history[self.history.len() - 1]);
            }
            Some(0) => {
                // Already at oldest entry
            }
            Some(idx) => {
                self.history_index = Some(idx - 1);
                self.input.set_value(&self.history[idx - 1]);
            }
        }
        self.dirty = true;
    }

    /// Navigate history down (newer)
    fn history_next(&mut self) {
        match self.history_index {
            None => {
                // Already at newest (current input)
            }
            Some(idx) if idx >= self.history.len() - 1 => {
                // Restore saved input
                self.history_index = None;
                if let Some(saved) = self.saved_input.take() {
                    self.input.set_value(&saved);
                } else {
                    self.input.clear();
                }
            }
            Some(idx) => {
                self.history_index = Some(idx + 1);
                self.input.set_value(&self.history[idx + 1]);
            }
        }
        self.dirty = true;
    }

    /// Update completions based on current input
    pub fn update_completions<E: CommandExecutor>(&mut self, executor: &E) {
        let partial = self.input.value();
        self.completions = executor.complete(partial, self.mode);
        self.completion_index = None;
        self.dirty = true;
    }

    /// Cycle to next completion
    fn complete_next(&mut self) {
        if self.completions.is_empty() {
            return;
        }

        match self.completion_index {
            None => {
                self.completion_index = Some(0);
                self.input.set_value(&self.completions[0]);
            }
            Some(idx) => {
                let next = (idx + 1) % self.completions.len();
                self.completion_index = Some(next);
                self.input.set_value(&self.completions[next]);
            }
        }
        self.dirty = true;
    }

    /// Cycle to previous completion
    fn complete_prev(&mut self) {
        if self.completions.is_empty() {
            return;
        }

        match self.completion_index {
            None => {
                let last = self.completions.len() - 1;
                self.completion_index = Some(last);
                self.input.set_value(&self.completions[last]);
            }
            Some(0) => {
                let last = self.completions.len() - 1;
                self.completion_index = Some(last);
                self.input.set_value(&self.completions[last]);
            }
            Some(idx) => {
                self.completion_index = Some(idx - 1);
                self.input.set_value(&self.completions[idx - 1]);
            }
        }
        self.dirty = true;
    }

    /// Get number of completions available
    pub fn completion_count(&self) -> usize {
        self.completions.len()
    }

    /// Get current completions
    pub fn completions(&self) -> &[String] {
        &self.completions
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler for CommandPalette {
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.active {
            return false;
        }

        match event {
            Event::Key(key) => match key {
                // Submit
                Key::Enter => {
                    // Mark as consumed - parent should call submit() to get the command
                    true
                }

                // Cancel
                Key::Esc => {
                    self.cancel();
                    true
                }

                // History navigation
                Key::Up => {
                    self.history_prev();
                    true
                }
                Key::Down => {
                    self.history_next();
                    true
                }

                // Completion
                Key::Tab => {
                    self.complete_next();
                    true
                }
                Key::BackTab => {
                    self.complete_prev();
                    true
                }

                // Ctrl+P/N for history (vi-style)
                Key::Ctrl('p') => {
                    self.history_prev();
                    true
                }
                Key::Ctrl('n') => {
                    self.history_next();
                    true
                }

                // Delegate to text input
                _ => {
                    let handled = self.input.handle_event(event);
                    if handled {
                        // Clear completions when input changes
                        self.completions.clear();
                        self.completion_index = None;
                    }
                    handled
                }
            },

            // Delegate paste to input
            Event::Paste(_) => self.input.handle_event(event),

            _ => false,
        }
    }

    fn on_focus(&mut self) {
        self.input.on_focus();
    }

    fn on_blur(&mut self) {
        self.input.on_blur();
    }
}

impl Component for CommandPalette {
    fn render(&mut self, renderer: &mut Renderer, bounds: Rect, ctx: &RenderContext) -> Result<()> {
        if !self.active {
            // When inactive, show last message or error if any
            if let Some(error) = &self.last_error {
                renderer.move_cursor(bounds.x, bounds.y)?;
                renderer.write_styled(error, "\x1b[31m")?; // Red
            } else if let Some(msg) = &self.last_message {
                renderer.move_cursor(bounds.x, bounds.y)?;
                renderer.write_text(msg)?;
            }
            self.dirty = false;
            return Ok(());
        }

        // Render the text input
        self.input.render(renderer, bounds, ctx)?;

        self.dirty = false;
        Ok(())
    }

    fn min_size(&self) -> (u16, u16) {
        (20, 1)
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.input.mark_dirty();
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.input.is_dirty()
    }

    fn name(&self) -> &str {
        "CommandPalette"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockExecutor;

    impl CommandExecutor for MockExecutor {
        fn execute(&mut self, _command: &str, _mode: CommandMode) -> CommandResult {
            CommandResult::Success(None)
        }

        fn complete(&self, partial: &str, _mode: CommandMode) -> Vec<String> {
            let commands = vec!["write", "quit", "wq", "help", "set"];
            commands
                .into_iter()
                .filter(|c| c.starts_with(partial))
                .map(|s| s.to_string())
                .collect()
        }
    }

    #[test]
    fn test_command_palette_creation() {
        let palette = CommandPalette::new();
        assert!(!palette.is_active());
        assert_eq!(palette.mode(), CommandMode::Ex);
    }

    #[test]
    fn test_activate_deactivate() {
        let mut palette = CommandPalette::new();

        palette.activate(CommandMode::Search);
        assert!(palette.is_active());
        assert_eq!(palette.mode(), CommandMode::Search);

        palette.deactivate();
        assert!(!palette.is_active());
    }

    #[test]
    fn test_mode_prompts() {
        assert_eq!(CommandMode::Ex.prompt(), ":");
        assert_eq!(CommandMode::Search.prompt(), "/");
        assert_eq!(CommandMode::SearchBack.prompt(), "?");
        assert_eq!(CommandMode::Shell.prompt(), "!");
    }

    #[test]
    fn test_history_navigation() {
        let mut palette = CommandPalette::new();

        // Add some history
        palette.activate(CommandMode::Ex);
        palette.input.set_value("cmd1");
        palette.submit();

        palette.activate(CommandMode::Ex);
        palette.input.set_value("cmd2");
        palette.submit();

        palette.activate(CommandMode::Ex);
        palette.input.set_value("cmd3");
        palette.submit();

        // Navigate history
        palette.activate(CommandMode::Ex);
        palette.history_prev();
        assert_eq!(palette.value(), "cmd3");

        palette.history_prev();
        assert_eq!(palette.value(), "cmd2");

        palette.history_next();
        assert_eq!(palette.value(), "cmd3");
    }

    #[test]
    fn test_completion() {
        let mut palette = CommandPalette::new();
        let executor = MockExecutor;

        palette.activate(CommandMode::Ex);
        palette.input.set_value("w");
        palette.update_completions(&executor);

        assert_eq!(palette.completions(), &["write", "wq"]);

        palette.complete_next();
        assert_eq!(palette.value(), "write");

        palette.complete_next();
        assert_eq!(palette.value(), "wq");

        palette.complete_prev();
        assert_eq!(palette.value(), "write");
    }
}

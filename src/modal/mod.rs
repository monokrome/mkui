//! Modal state machine - Vim-like modal editing support
//!
//! Provides a generic modal editing system with:
//! - Normal, Visual, Insert, and Command modes
//! - Count accumulator (e.g., `3dw` = delete 3 words)
//! - Pending operator tracking (e.g., `d` waits for motion)
//! - Named registers for copy/paste
//! - Extensible motion and operator systems

mod motions;

pub use motions::{Motion, Operator};

use std::collections::HashMap;

/// Operating mode for modal editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    /// Default mode for navigation and commands
    Normal,
    /// Visual selection mode with a sub-type
    Visual(VisualMode),
    /// Text insertion mode
    Insert,
    /// Command-line input mode (`:` prefix)
    Command,
    /// Incremental search mode
    Search(SearchDirection),
}

impl Mode {
    /// Returns true if this is any visual selection mode
    pub fn is_visual(&self) -> bool {
        matches!(self, Mode::Visual(_))
    }

    /// Returns the display name for the status line
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Visual(VisualMode::Character) => "VISUAL",
            Mode::Visual(VisualMode::Line) => "V-LINE",
            Mode::Visual(VisualMode::Block) => "V-BLOCK",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
            Mode::Search(SearchDirection::Forward) => "SEARCH",
            Mode::Search(SearchDirection::Backward) => "SEARCH?",
        }
    }
}

/// Visual selection mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualMode {
    /// Character-wise selection (`v`)
    Character,
    /// Line-wise selection (`V`)
    Line,
    /// Block/column selection (`Ctrl-V`)
    Block,
}

/// Search direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    /// Search forward through the document (`/`)
    Forward,
    /// Search backward through the document (`?`)
    Backward,
}

/// Modal state machine
///
/// Tracks the current editing mode, pending operations, and accumulated state
/// like counts and registers.
#[derive(Debug)]
pub struct ModalState {
    mode: Mode,
    count: Option<usize>,
    pending_operator: Option<Operator>,
    pending_keys: String,
    register: char,
    last_search: Option<String>,
    last_search_direction: SearchDirection,
    last_find_char: Option<(char, bool, bool)>,
    registers: HashMap<char, ()>,
}

impl Default for ModalState {
    fn default() -> Self {
        Self::new()
    }
}

impl ModalState {
    /// Creates a new modal state in Normal mode
    pub fn new() -> Self {
        ModalState {
            mode: Mode::Normal,
            count: None,
            pending_operator: None,
            pending_keys: String::new(),
            register: '"',
            last_search: None,
            last_search_direction: SearchDirection::Forward,
            last_find_char: None,
            registers: HashMap::new(),
        }
    }

    /// Returns the current editing mode
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Sets the editing mode, clearing pending state for non-visual modes
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if !mode.is_visual() {
            self.clear_pending();
        }
    }

    /// Transitions to Normal mode
    pub fn enter_normal(&mut self) {
        self.set_mode(Mode::Normal);
    }

    /// Transitions to Insert mode
    pub fn enter_insert(&mut self) {
        self.set_mode(Mode::Insert);
    }

    /// Transitions to character-wise Visual mode
    pub fn enter_visual(&mut self) {
        self.set_mode(Mode::Visual(VisualMode::Character));
    }

    /// Transitions to line-wise Visual mode
    pub fn enter_visual_line(&mut self) {
        self.set_mode(Mode::Visual(VisualMode::Line));
    }

    /// Transitions to block-wise Visual mode
    pub fn enter_visual_block(&mut self) {
        self.set_mode(Mode::Visual(VisualMode::Block));
    }

    /// Transitions to Command mode
    pub fn enter_command(&mut self) {
        self.set_mode(Mode::Command);
    }

    /// Transitions to Search mode with the given direction
    pub fn enter_search(&mut self, direction: SearchDirection) {
        self.set_mode(Mode::Search(direction));
    }

    /// Get current count (defaults to 1)
    pub fn count(&self) -> usize {
        self.count.unwrap_or(1)
    }

    /// Returns the raw count, or `None` if no count was entered
    pub fn count_opt(&self) -> Option<usize> {
        self.count
    }

    /// Appends a digit to the current count accumulator
    pub fn accumulate_count(&mut self, digit: char) {
        if let Some(d) = digit.to_digit(10) {
            let current = self.count.unwrap_or(0);
            self.count = Some(current * 10 + d as usize);
        }
    }

    /// Resets the count accumulator to `None`
    pub fn clear_count(&mut self) {
        self.count = None;
    }

    /// Returns a reference to the pending operator, if any
    pub fn pending_operator(&self) -> Option<&Operator> {
        self.pending_operator.as_ref()
    }

    /// Sets the operator waiting for a motion (e.g., `d` awaiting `w`)
    pub fn set_pending_operator(&mut self, op: Operator) {
        self.pending_operator = Some(op);
    }

    /// Takes and returns the pending operator, leaving `None` in its place
    pub fn take_pending_operator(&mut self) -> Option<Operator> {
        self.pending_operator.take()
    }

    /// Returns true if an operator is waiting for a motion
    pub fn has_pending_operator(&self) -> bool {
        self.pending_operator.is_some()
    }

    /// Returns the buffered incomplete key sequence
    pub fn pending_keys(&self) -> &str {
        &self.pending_keys
    }

    /// Appends a character to the pending key buffer
    pub fn push_pending_key(&mut self, c: char) {
        self.pending_keys.push(c);
    }

    /// Clears the pending key buffer
    pub fn clear_pending_keys(&mut self) {
        self.pending_keys.clear();
    }

    /// Returns the currently selected register name
    pub fn register(&self) -> char {
        self.register
    }

    /// Selects a named register and marks it as used
    pub fn set_register(&mut self, register: char) {
        self.register = register;
        self.registers.insert(register, ());
    }

    /// Resets the register selection to the default (`"`)
    pub fn reset_register(&mut self) {
        self.register = '"';
    }

    /// Clear all pending state (count, operator, keys)
    pub fn clear_pending(&mut self) {
        self.count = None;
        self.pending_operator = None;
        self.pending_keys.clear();
        self.register = '"';
    }

    /// Records the last search pattern and direction
    pub fn set_last_search(&mut self, pattern: String, direction: SearchDirection) {
        self.last_search = Some(pattern);
        self.last_search_direction = direction;
    }

    /// Returns the last search pattern, if any
    pub fn last_search(&self) -> Option<&str> {
        self.last_search.as_deref()
    }

    /// Returns the direction of the last search
    pub fn last_search_direction(&self) -> SearchDirection {
        self.last_search_direction
    }

    /// Records the last `f`/`F`/`t`/`T` character find
    pub fn set_last_find(&mut self, c: char, is_till: bool, is_backward: bool) {
        self.last_find_char = Some((c, is_till, is_backward));
    }

    /// Returns the last character find as `(char, is_till, is_backward)`
    pub fn last_find(&self) -> Option<(char, bool, bool)> {
        self.last_find_char
    }

    /// Get status line display string
    pub fn status(&self) -> String {
        let mut s = String::new();

        if let Some(count) = self.count {
            s.push_str(&count.to_string());
        }

        if let Some(op) = &self.pending_operator {
            s.push(op.to_char());
        }

        s.push_str(&self.pending_keys);

        s
    }
}

/// Trait for components that handle modal editing
pub trait ModalHandler {
    /// Executes a motion, returning true if the cursor moved
    fn execute_motion(&mut self, motion: Motion, count: usize) -> bool;
    /// Executes an operator over a motion range, returning true on success
    fn execute_operator(&mut self, op: Operator, motion: Motion, count: usize) -> bool;
    /// Called when entering insert mode
    fn enter_insert(&mut self);
    /// Called when leaving insert mode
    fn exit_insert(&mut self);
    /// Returns the current cursor position
    fn position(&self) -> usize;
    /// Sets the cursor position
    fn set_position(&mut self, pos: usize);
}

/// Result of processing a key in modal mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyResult {
    /// Key was fully handled
    Consumed,
    /// Key added to an incomplete sequence, awaiting more input
    Pending,
    /// Key resolved to a motion
    Motion(Motion),
    /// Key completed an operator-motion pair
    Operation(Operator, Motion),
    /// Key triggered a mode change
    ModeChange(Mode),
    /// Key was not recognized in the current context
    Unhandled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_state_creation() {
        let state = ModalState::new();
        assert_eq!(state.mode(), Mode::Normal);
        assert_eq!(state.count(), 1);
        assert!(!state.has_pending_operator());
    }

    #[test]
    fn test_mode_transitions() {
        let mut state = ModalState::new();

        state.enter_insert();
        assert_eq!(state.mode(), Mode::Insert);

        state.enter_normal();
        assert_eq!(state.mode(), Mode::Normal);

        state.enter_visual();
        assert_eq!(state.mode(), Mode::Visual(VisualMode::Character));

        state.enter_visual_line();
        assert_eq!(state.mode(), Mode::Visual(VisualMode::Line));

        state.enter_command();
        assert_eq!(state.mode(), Mode::Command);
    }

    #[test]
    fn test_count_accumulation() {
        let mut state = ModalState::new();

        state.accumulate_count('3');
        assert_eq!(state.count(), 3);

        state.accumulate_count('2');
        assert_eq!(state.count(), 32);

        state.clear_count();
        assert_eq!(state.count(), 1);
    }

    #[test]
    fn test_pending_operator() {
        let mut state = ModalState::new();

        state.set_pending_operator(Operator::Delete);
        assert!(state.has_pending_operator());

        let op = state.take_pending_operator();
        assert_eq!(op, Some(Operator::Delete));
        assert!(!state.has_pending_operator());
    }

    #[test]
    fn test_register_handling() {
        let mut state = ModalState::new();

        assert_eq!(state.register(), '"');

        state.set_register('a');
        assert_eq!(state.register(), 'a');

        state.reset_register();
        assert_eq!(state.register(), '"');
    }

    #[test]
    fn test_status_display() {
        let mut state = ModalState::new();

        state.accumulate_count('3');
        state.set_pending_operator(Operator::Delete);
        state.push_pending_key('i');

        assert_eq!(state.status(), "3di");
    }

    #[test]
    fn test_clear_pending() {
        let mut state = ModalState::new();

        state.accumulate_count('5');
        state.set_pending_operator(Operator::Yank);
        state.push_pending_key('w');
        state.set_register('a');

        state.clear_pending();

        assert_eq!(state.count(), 1);
        assert!(!state.has_pending_operator());
        assert!(state.pending_keys().is_empty());
        assert_eq!(state.register(), '"');
    }

    #[test]
    fn test_mode_names() {
        assert_eq!(Mode::Normal.name(), "NORMAL");
        assert_eq!(Mode::Insert.name(), "INSERT");
        assert_eq!(Mode::Visual(VisualMode::Character).name(), "VISUAL");
        assert_eq!(Mode::Visual(VisualMode::Line).name(), "V-LINE");
        assert_eq!(Mode::Visual(VisualMode::Block).name(), "V-BLOCK");
        assert_eq!(Mode::Command.name(), "COMMAND");
    }
}

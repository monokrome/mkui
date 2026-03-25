//! Event system - keyboard, mouse, and terminal events

use anyhow::Result;
use std::time::Duration;

/// Keyboard key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Regular character key
    Char(char),
    /// Function key (F1-F12)
    F(u8),
    /// Ctrl + character combination
    Ctrl(char),
    /// Alt + character combination
    Alt(char),
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Home key
    Home,
    /// End key
    End,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Shift+Tab (reverse tab)
    BackTab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Insert key
    Insert,
    /// Enter/Return key
    Enter,
    /// Tab key
    Tab,
    /// Escape key
    Esc,
    /// Null/unknown key
    Null,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button (scroll wheel click)
    Middle,
}

/// Mouse event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    /// Button press at (col, row)
    Press(MouseButton, u16, u16),
    /// Button release at (col, row)
    Release(u16, u16),
    /// Drag/hold at (col, row)
    Hold(u16, u16),
    /// Scroll up at (col, row)
    ScrollUp(u16, u16),
    /// Scroll down at (col, row)
    ScrollDown(u16, u16),
}

/// The backend-specific original event, accessible if you need it
#[derive(Debug, Clone)]
pub enum RawEvent {
    /// Original crossterm event
    #[cfg(feature = "tui")]
    Crossterm(crossterm::event::Event),
    /// Original winit window event (cloned to owned types)
    #[cfg(feature = "gui")]
    Winit(winit::event::WindowEvent),
}

/// Classified event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// Keyboard event
    Key(Key),
    /// Mouse event
    Mouse(MouseEvent),
    /// Surface resized (new cols, new rows)
    Resize(u16, u16),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// Paste event
    Paste(String),
}

/// A UI event with both an abstracted kind and the original backend event
#[derive(Debug, Clone)]
pub struct Event {
    /// The classified event
    pub kind: EventKind,
    /// The original backend-specific event, if available
    pub raw: Option<RawEvent>,
}

impl Event {
    /// Create an event with no raw backing
    pub fn new(kind: EventKind) -> Self {
        Self { kind, raw: None }
    }

    /// Create an event with a raw backing event
    pub fn with_raw(kind: EventKind, raw: RawEvent) -> Self {
        Self {
            kind,
            raw: Some(raw),
        }
    }

    /// Create a key event
    pub fn key(key: Key) -> Self {
        Self::new(EventKind::Key(key))
    }

    /// Create a mouse event
    pub fn mouse(mouse: MouseEvent) -> Self {
        Self::new(EventKind::Mouse(mouse))
    }

    /// Create a resize event
    pub fn resize(cols: u16, rows: u16) -> Self {
        Self::new(EventKind::Resize(cols, rows))
    }
}

/// Event handler trait for components
pub trait EventHandler {
    /// Handle an event, return true if consumed (stops propagation)
    fn handle_event(&mut self, _event: &Event) -> bool {
        false
    }

    /// Called when component gains focus
    fn on_focus(&mut self) {}

    /// Called when component loses focus
    fn on_blur(&mut self) {}
}

#[cfg(feature = "tui")]
/// Event polling and conversion from crossterm events
pub struct EventPoller;

#[cfg(feature = "tui")]
impl EventPoller {
    /// Create a new event poller
    pub fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;

        // Try to enable mouse and focus, but don't fail if not available
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableFocusChange,
        );

        Ok(EventPoller)
    }

    /// Poll for next event with timeout (use sparingly - prefer read() or wait())
    pub fn poll(&self, timeout: Duration) -> Result<Option<Event>> {
        if crossterm::event::poll(timeout)? {
            let raw = crossterm::event::read()?;
            Ok(Some(convert_crossterm_event(raw)))
        } else {
            Ok(None)
        }
    }

    /// Block and wait for next event - PREFERRED for event-driven apps
    pub fn read(&self) -> Result<Event> {
        let raw = crossterm::event::read()?;
        Ok(convert_crossterm_event(raw))
    }

    /// Check if an event is available without blocking
    pub fn has_event(&self) -> Result<bool> {
        Ok(crossterm::event::poll(Duration::ZERO)?)
    }

    /// Wait for event OR timeout, whichever comes first
    /// Returns None on timeout, Some(event) if event arrived
    pub fn wait(&self, timeout: Duration) -> Result<Option<Event>> {
        self.poll(timeout)
    }
}

/// Frame timing for animation
#[cfg(feature = "tui")]
pub struct FrameTimer {
    frame_duration: Duration,
    last_frame: std::time::Instant,
}

#[cfg(feature = "tui")]
impl FrameTimer {
    /// Create a new frame timer targeting the given FPS
    pub fn new(fps: u32) -> Self {
        Self {
            frame_duration: Duration::from_nanos(1_000_000_000 / fps as u64),
            last_frame: std::time::Instant::now(),
        }
    }

    /// Time until next frame (zero if frame is due)
    pub fn time_to_next_frame(&self) -> Duration {
        let elapsed = self.last_frame.elapsed();
        if elapsed >= self.frame_duration {
            Duration::ZERO
        } else {
            self.frame_duration - elapsed
        }
    }

    /// Mark frame as rendered, returns true if we're on time
    pub fn tick(&mut self) -> bool {
        let elapsed = self.last_frame.elapsed();
        self.last_frame = std::time::Instant::now();
        elapsed <= self.frame_duration
    }

    /// Check if a frame is due now
    pub fn frame_due(&self) -> bool {
        self.last_frame.elapsed() >= self.frame_duration
    }
}

#[cfg(feature = "tui")]
impl Drop for EventPoller {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableFocusChange,
        );
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[cfg(feature = "tui")]
/// Convert crossterm event to mkui Event
fn convert_crossterm_event(event: crossterm::event::Event) -> Event {
    use crossterm::event::{Event as CEvent, KeyEvent, MouseEventKind};

    let kind = match &event {
        CEvent::Key(KeyEvent {
            code, modifiers, ..
        }) => EventKind::Key(convert_key(*code, *modifiers)),
        CEvent::Mouse(me) => {
            let (col, row) = (me.column, me.row);
            let mouse_event = match me.kind {
                MouseEventKind::Down(btn) => match btn {
                    crossterm::event::MouseButton::Left => {
                        MouseEvent::Press(MouseButton::Left, col, row)
                    }
                    crossterm::event::MouseButton::Right => {
                        MouseEvent::Press(MouseButton::Right, col, row)
                    }
                    crossterm::event::MouseButton::Middle => {
                        MouseEvent::Press(MouseButton::Middle, col, row)
                    }
                },
                MouseEventKind::Up(_) => MouseEvent::Release(col, row),
                MouseEventKind::Drag(_) => MouseEvent::Hold(col, row),
                MouseEventKind::Moved => MouseEvent::Hold(col, row),
                MouseEventKind::ScrollUp => MouseEvent::ScrollUp(col, row),
                MouseEventKind::ScrollDown => MouseEvent::ScrollDown(col, row),
                _ => MouseEvent::Release(col, row),
            };
            EventKind::Mouse(mouse_event)
        }
        CEvent::Resize(cols, rows) => EventKind::Resize(*cols, *rows),
        CEvent::FocusGained => EventKind::FocusGained,
        CEvent::FocusLost => EventKind::FocusLost,
        CEvent::Paste(data) => EventKind::Paste(data.clone()),
    };

    Event::with_raw(kind, RawEvent::Crossterm(event))
}

#[cfg(feature = "tui")]
/// Convert crossterm key code to our Key type
fn convert_key(code: crossterm::event::KeyCode, mods: crossterm::event::KeyModifiers) -> Key {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Handle Ctrl modifier
    if mods.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = code {
            return Key::Ctrl(c);
        }
    }

    // Handle Alt modifier
    if mods.contains(KeyModifiers::ALT) {
        if let KeyCode::Char(c) = code {
            return Key::Alt(c);
        }
    }

    // Regular keys
    match code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::F(n) => Key::F(n),
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Esc,
        KeyCode::Null => Key::Null,
        _ => Key::Null,
    }
}

/// Convert a winit WindowEvent to an mkui Event
#[cfg(feature = "gui")]
pub fn convert_winit_event(event: &winit::event::WindowEvent) -> Option<Event> {
    use winit::event::{ElementState, WindowEvent};
    use winit::keyboard::{Key as WKey, NamedKey};

    let kind = match event {
        WindowEvent::KeyboardInput { event: key_event, .. }
            if key_event.state == ElementState::Pressed =>
        {
            let key = match &key_event.logical_key {
                WKey::Named(named) => match named {
                    NamedKey::Escape => Key::Esc,
                    NamedKey::Enter => Key::Enter,
                    NamedKey::Tab => Key::Tab,
                    NamedKey::Backspace => Key::Backspace,
                    NamedKey::Delete => Key::Delete,
                    NamedKey::Insert => Key::Insert,
                    NamedKey::Home => Key::Home,
                    NamedKey::End => Key::End,
                    NamedKey::PageUp => Key::PageUp,
                    NamedKey::PageDown => Key::PageDown,
                    NamedKey::ArrowUp => Key::Up,
                    NamedKey::ArrowDown => Key::Down,
                    NamedKey::ArrowLeft => Key::Left,
                    NamedKey::ArrowRight => Key::Right,
                    NamedKey::F1 => Key::F(1),
                    NamedKey::F2 => Key::F(2),
                    NamedKey::F3 => Key::F(3),
                    NamedKey::F4 => Key::F(4),
                    NamedKey::F5 => Key::F(5),
                    NamedKey::F6 => Key::F(6),
                    NamedKey::F7 => Key::F(7),
                    NamedKey::F8 => Key::F(8),
                    NamedKey::F9 => Key::F(9),
                    NamedKey::F10 => Key::F(10),
                    NamedKey::F11 => Key::F(11),
                    NamedKey::F12 => Key::F(12),
                    _ => return None,
                },
                WKey::Character(c) => {
                    let mut chars = c.chars();
                    match chars.next() {
                        Some(ch) if chars.next().is_none() => Key::Char(ch),
                        _ => return None,
                    }
                }
                _ => return None,
            };
            EventKind::Key(key)
        }
        WindowEvent::Resized(size) => {
            // GUI reports pixel sizes — callers convert to cells as needed
            EventKind::Resize(size.width as u16, size.height as u16)
        }
        WindowEvent::Focused(true) => EventKind::FocusGained,
        WindowEvent::Focused(false) => EventKind::FocusLost,
        _ => return None,
    };

    Some(Event::with_raw(kind, RawEvent::Winit(event.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_variants() {
        let k = Key::Char('a');
        assert_eq!(k, Key::Char('a'));

        let k2 = Key::Ctrl('c');
        assert_eq!(k2, Key::Ctrl('c'));
    }

    #[test]
    fn test_event_types() {
        let e = Event::key(Key::Enter);
        match e.kind {
            EventKind::Key(Key::Enter) => {}
            other => panic!("expected Key(Enter), got {:?}", other),
        }
    }

    #[test]
    fn test_event_raw_none() {
        let e = Event::new(EventKind::FocusGained);
        assert!(e.raw.is_none());
    }
}

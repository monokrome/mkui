//! Event system - keyboard, mouse, and terminal events

use anyhow::Result;
use std::time::Duration;

/// Keyboard key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    F(u8),
    Ctrl(char),
    Alt(char),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    BackTab,
    Backspace,
    Delete,
    Insert,
    Enter,
    Tab,
    Esc,
    Null,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Mouse event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Press(MouseButton, u16, u16), // button, col, row
    Release(u16, u16),            // col, row
    Hold(u16, u16),               // col, row (drag)
    ScrollUp(u16, u16),           // col, row
    ScrollDown(u16, u16),         // col, row
}

/// UI events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Keyboard event
    Key(Key),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resized (new cols, new rows)
    Resize(u16, u16),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// Paste event
    Paste(String),
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

/// Event polling and conversion from crossterm events
pub struct EventPoller {
    _enabled: bool,
}

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

        Ok(EventPoller { _enabled: true })
    }

    /// Poll for next event with timeout (use sparingly - prefer read() or wait())
    pub fn poll(&self, timeout: Duration) -> Result<Option<Event>> {
        if crossterm::event::poll(timeout)? {
            let event = crossterm::event::read()?;
            Ok(Some(convert_crossterm_event(event)))
        } else {
            Ok(None)
        }
    }

    /// Block and wait for next event - PREFERRED for event-driven apps
    pub fn read(&self) -> Result<Event> {
        let event = crossterm::event::read()?;
        Ok(convert_crossterm_event(event))
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
pub struct FrameTimer {
    _target_fps: u32,
    frame_duration: Duration,
    last_frame: std::time::Instant,
}

impl FrameTimer {
    pub fn new(fps: u32) -> Self {
        Self {
            _target_fps: fps,
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

/// Convert crossterm event to our Event type
fn convert_crossterm_event(event: crossterm::event::Event) -> Event {
    use crossterm::event::{Event as CEvent, KeyEvent, MouseEventKind};

    match event {
        CEvent::Key(KeyEvent {
            code, modifiers, ..
        }) => Event::Key(convert_key(code, modifiers)),
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
                _ => MouseEvent::Release(col, row), // fallback
            };
            Event::Mouse(mouse_event)
        }
        CEvent::Resize(cols, rows) => Event::Resize(cols, rows),
        CEvent::FocusGained => Event::FocusGained,
        CEvent::FocusLost => Event::FocusLost,
        CEvent::Paste(data) => Event::Paste(data),
    }
}

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
        let e = Event::Key(Key::Enter);
        match e {
            Event::Key(Key::Enter) => {}
            other => panic!("expected Key(Enter), got {:?}", other),
        }
    }
}

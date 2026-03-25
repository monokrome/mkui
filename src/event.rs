//! Event system - keyboard, mouse, and terminal events
//!
//! The event model targets the highest level of flexibility (GUI/winit) and
//! virtualizes what terminal backends can't provide. This means components
//! get a consistent, rich event model regardless of backend.

use anyhow::Result;
use std::time::Duration;

/// Keyboard key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Regular character key
    Char(char),
    /// Function key (F1-F24)
    F(u8),
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
    /// Space key (distinct from Char(' ') for modifier combos)
    Space,
    /// Null/unknown key
    Null,
}

/// Keyboard modifier state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Shift key held
    pub shift: bool,
    /// Control key held
    pub ctrl: bool,
    /// Alt/Option key held
    pub alt: bool,
    /// Super/Meta/Windows/Command key held
    pub super_key: bool,
    /// Hyper key held (rare, some terminals support it)
    pub hyper: bool,
}

impl Modifiers {
    /// No modifiers pressed
    pub fn none() -> Self {
        Self::default()
    }

    /// Check if any modifier is active
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.super_key || self.hyper
    }
}

/// Whether a key was pressed, released, or is repeating
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    /// Key was just pressed
    Pressed,
    /// Key was released
    Released,
    /// Key is being held and repeating
    Repeat,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button (scroll wheel click)
    Middle,
    /// Back/side button (mouse4)
    Back,
    /// Forward/side button (mouse5)
    Forward,
    /// Other button by number
    Other(u16),
}

/// Mouse event types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEvent {
    /// Button state changed at position
    Button {
        /// Which button
        button: MouseButton,
        /// Pressed or released
        state: KeyState,
        /// Column position
        col: u16,
        /// Row position
        row: u16,
        /// Modifier keys held during click
        modifiers: Modifiers,
    },
    /// Mouse cursor moved to position
    Moved {
        /// Column position
        col: u16,
        /// Row position
        row: u16,
    },
    /// Scroll wheel
    Scroll {
        /// Horizontal scroll delta (positive = right)
        delta_x: f32,
        /// Vertical scroll delta (positive = down/towards user)
        delta_y: f32,
        /// Column position
        col: u16,
        /// Row position
        row: u16,
        /// Modifier keys held during scroll
        modifiers: Modifiers,
    },
}

/// The backend-specific original event, accessible if you need it
#[derive(Debug, Clone)]
pub enum RawEvent {
    /// Original crossterm event
    #[cfg(feature = "tui")]
    Crossterm(crossterm::event::Event),
    /// Original winit window event
    #[cfg(feature = "gui")]
    Winit(winit::event::WindowEvent),
}

/// Classified event type
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    /// Keyboard event
    Key {
        /// Which key
        key: Key,
        /// Press/release/repeat state
        state: KeyState,
        /// Modifier keys held
        modifiers: Modifiers,
        /// Text produced by this keypress (for text input; may differ from key due to IME/layout)
        text: Option<String>,
    },
    /// Mouse event
    Mouse(MouseEvent),
    /// Surface resized (new cols, new rows)
    Resize(u16, u16),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// Paste event (bracketed paste in terminals, clipboard in GUI)
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

    /// Create a key press event with no modifiers
    pub fn key(key: Key) -> Self {
        Self::new(EventKind::Key {
            key,
            state: KeyState::Pressed,
            modifiers: Modifiers::none(),
            text: None,
        })
    }

    /// Create a key press event with modifiers
    pub fn key_with_mods(key: Key, modifiers: Modifiers) -> Self {
        Self::new(EventKind::Key {
            key,
            state: KeyState::Pressed,
            modifiers,
            text: None,
        })
    }

    /// Create a resize event
    pub fn resize(cols: u16, rows: u16) -> Self {
        Self::new(EventKind::Resize(cols, rows))
    }

    /// Check if this is a key press matching a specific key (ignoring modifiers)
    pub fn is_key(&self, key: Key) -> bool {
        matches!(&self.kind, EventKind::Key { key: k, state: KeyState::Pressed, .. } if *k == key)
    }

    /// Check if this is a key press with specific modifiers
    pub fn is_key_with_mods(&self, key: Key, modifiers: Modifiers) -> bool {
        matches!(&self.kind, EventKind::Key { key: k, state: KeyState::Pressed, modifiers: m, .. } if *k == key && *m == modifiers)
    }
}

impl EventKind {
    /// Check if this is a key press of a specific key (ignoring modifiers)
    pub fn is_key_press(&self, key: Key) -> bool {
        matches!(self, EventKind::Key { key: k, state: KeyState::Pressed | KeyState::Repeat, .. } if *k == key)
    }

    /// Check if this is a Ctrl+key press
    pub fn is_ctrl(&self, ch: char) -> bool {
        matches!(self, EventKind::Key {
            key: Key::Char(c),
            state: KeyState::Pressed | KeyState::Repeat,
            modifiers: Modifiers { ctrl: true, .. },
            ..
        } if *c == ch)
    }

    /// Check if this is an Alt+key press
    pub fn is_alt(&self, ch: char) -> bool {
        matches!(self, EventKind::Key {
            key: Key::Char(c),
            state: KeyState::Pressed | KeyState::Repeat,
            modifiers: Modifiers { alt: true, .. },
            ..
        } if *c == ch)
    }

    /// Extract the key from a key press event (ignoring release)
    pub fn pressed_key(&self) -> Option<&Key> {
        match self {
            EventKind::Key { key, state: KeyState::Pressed | KeyState::Repeat, .. } => Some(key),
            _ => None,
        }
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

// -- TUI backend: crossterm conversion --

#[cfg(feature = "tui")]
/// Event polling and conversion from crossterm events
pub struct EventPoller;

#[cfg(feature = "tui")]
impl EventPoller {
    /// Create a new event poller
    pub fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;

        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableFocusChange,
        );

        Ok(EventPoller)
    }

    /// Poll for next event with timeout
    pub fn poll(&self, timeout: Duration) -> Result<Option<Event>> {
        if crossterm::event::poll(timeout)? {
            let raw = crossterm::event::read()?;
            Ok(Some(convert_crossterm_event(raw)))
        } else {
            Ok(None)
        }
    }

    /// Block and wait for next event
    pub fn read(&self) -> Result<Event> {
        let raw = crossterm::event::read()?;
        Ok(convert_crossterm_event(raw))
    }

    /// Check if an event is available without blocking
    pub fn has_event(&self) -> Result<bool> {
        Ok(crossterm::event::poll(Duration::ZERO)?)
    }

    /// Wait for event OR timeout, whichever comes first
    pub fn wait(&self, timeout: Duration) -> Result<Option<Event>> {
        self.poll(timeout)
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
fn crossterm_modifiers(mods: crossterm::event::KeyModifiers) -> Modifiers {
    use crossterm::event::KeyModifiers;
    Modifiers {
        shift: mods.contains(KeyModifiers::SHIFT),
        ctrl: mods.contains(KeyModifiers::CONTROL),
        alt: mods.contains(KeyModifiers::ALT),
        super_key: mods.contains(KeyModifiers::SUPER),
        hyper: mods.contains(KeyModifiers::HYPER),
    }
}

#[cfg(feature = "tui")]
fn convert_crossterm_key(code: crossterm::event::KeyCode) -> Key {
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Char(' ') => Key::Space,
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
        KeyCode::BackTab => Key::Tab, // BackTab is Shift+Tab — modifier handles it
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

#[cfg(feature = "tui")]
fn convert_crossterm_event(event: crossterm::event::Event) -> Event {
    use crossterm::event::{Event as CEvent, KeyEvent as CKeyEvent, KeyEventKind, MouseEventKind};

    let kind = match &event {
        CEvent::Key(CKeyEvent {
            code,
            modifiers: mods,
            kind: key_kind,
            ..
        }) => {
            let mut modifiers = crossterm_modifiers(*mods);
            let key = convert_crossterm_key(*code);

            // BackTab means shift was held
            if *code == crossterm::event::KeyCode::BackTab {
                modifiers.shift = true;
            }

            let state = match key_kind {
                KeyEventKind::Press => KeyState::Pressed,
                KeyEventKind::Release => KeyState::Released,
                KeyEventKind::Repeat => KeyState::Repeat,
            };

            let text = match key {
                Key::Char(c) if !modifiers.ctrl && !modifiers.alt => Some(c.to_string()),
                Key::Space if !modifiers.ctrl && !modifiers.alt => Some(" ".to_string()),
                _ => None,
            };

            EventKind::Key {
                key,
                state,
                modifiers,
                text,
            }
        }
        CEvent::Mouse(me) => {
            let mods = crossterm_modifiers(me.modifiers);
            let (col, row) = (me.column, me.row);

            let mouse = match me.kind {
                MouseEventKind::Down(btn) => MouseEvent::Button {
                    button: convert_crossterm_mouse_button(btn),
                    state: KeyState::Pressed,
                    col,
                    row,
                    modifiers: mods,
                },
                MouseEventKind::Up(btn) => MouseEvent::Button {
                    button: convert_crossterm_mouse_button(btn),
                    state: KeyState::Released,
                    col,
                    row,
                    modifiers: mods,
                },
                MouseEventKind::Drag(_) | MouseEventKind::Moved => {
                    MouseEvent::Moved { col, row }
                }
                MouseEventKind::ScrollUp => MouseEvent::Scroll {
                    delta_x: 0.0,
                    delta_y: -1.0,
                    col,
                    row,
                    modifiers: mods,
                },
                MouseEventKind::ScrollDown => MouseEvent::Scroll {
                    delta_x: 0.0,
                    delta_y: 1.0,
                    col,
                    row,
                    modifiers: mods,
                },
                MouseEventKind::ScrollLeft => MouseEvent::Scroll {
                    delta_x: -1.0,
                    delta_y: 0.0,
                    col,
                    row,
                    modifiers: mods,
                },
                MouseEventKind::ScrollRight => MouseEvent::Scroll {
                    delta_x: 1.0,
                    delta_y: 0.0,
                    col,
                    row,
                    modifiers: mods,
                },
            };
            EventKind::Mouse(mouse)
        }
        CEvent::Resize(cols, rows) => EventKind::Resize(*cols, *rows),
        CEvent::FocusGained => EventKind::FocusGained,
        CEvent::FocusLost => EventKind::FocusLost,
        CEvent::Paste(data) => EventKind::Paste(data.clone()),
    };

    Event::with_raw(kind, RawEvent::Crossterm(event))
}

#[cfg(feature = "tui")]
fn convert_crossterm_mouse_button(btn: crossterm::event::MouseButton) -> MouseButton {
    match btn {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}

// -- GUI backend: winit conversion --

/// Convert a winit WindowEvent to an mkui Event
#[cfg(feature = "gui")]
pub fn convert_winit_event(event: &winit::event::WindowEvent) -> Option<Event> {
    use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
    use winit::keyboard::{Key as WKey, NamedKey};

    let kind = match event {
        WindowEvent::KeyboardInput { event: key_event, .. } => {
            let state = match key_event.state {
                ElementState::Pressed if key_event.repeat => KeyState::Repeat,
                ElementState::Pressed => KeyState::Pressed,
                ElementState::Released => KeyState::Released,
            };

            let key = match &key_event.logical_key {
                WKey::Named(named) => match named {
                    NamedKey::Escape => Key::Esc,
                    NamedKey::Enter => Key::Enter,
                    NamedKey::Tab => Key::Tab,
                    NamedKey::Space => Key::Space,
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
                    NamedKey::F13 => Key::F(13),
                    NamedKey::F14 => Key::F(14),
                    NamedKey::F15 => Key::F(15),
                    NamedKey::F16 => Key::F(16),
                    NamedKey::F17 => Key::F(17),
                    NamedKey::F18 => Key::F(18),
                    NamedKey::F19 => Key::F(19),
                    NamedKey::F20 => Key::F(20),
                    NamedKey::F21 => Key::F(21),
                    NamedKey::F22 => Key::F(22),
                    NamedKey::F23 => Key::F(23),
                    NamedKey::F24 => Key::F(24),
                    _ => Key::Null,
                },
                WKey::Character(c) => {
                    let mut chars = c.chars();
                    match chars.next() {
                        Some(' ') if chars.next().is_none() => Key::Space,
                        Some(ch) if chars.next().is_none() => Key::Char(ch),
                        _ => Key::Null,
                    }
                }
                _ => Key::Null,
            };

            if key == Key::Null {
                return None;
            }

            // winit doesn't give us modifier state on KeyEvent directly;
            // we'd need to track ModifiersChanged events for full accuracy.
            // For now, infer from the key itself.
            let modifiers = Modifiers::none();

            let text = key_event.text.as_ref().map(|s| s.to_string());

            EventKind::Key {
                key,
                state,
                modifiers,
                text,
            }
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let btn = match button {
                winit::event::MouseButton::Left => MouseButton::Left,
                winit::event::MouseButton::Right => MouseButton::Right,
                winit::event::MouseButton::Middle => MouseButton::Middle,
                winit::event::MouseButton::Back => MouseButton::Back,
                winit::event::MouseButton::Forward => MouseButton::Forward,
                winit::event::MouseButton::Other(n) => MouseButton::Other(*n),
            };
            let key_state = match state {
                ElementState::Pressed => KeyState::Pressed,
                ElementState::Released => KeyState::Released,
            };
            // Position will be 0,0 — caller should track CursorMoved for position
            EventKind::Mouse(MouseEvent::Button {
                button: btn,
                state: key_state,
                col: 0,
                row: 0,
                modifiers: Modifiers::none(),
            })
        }
        WindowEvent::CursorMoved { position, .. } => {
            // Pixel positions — caller converts to cells
            EventKind::Mouse(MouseEvent::Moved {
                col: position.x as u16,
                row: position.y as u16,
            })
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (dx, dy) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
            };
            EventKind::Mouse(MouseEvent::Scroll {
                delta_x: dx,
                delta_y: dy,
                col: 0,
                row: 0,
                modifiers: Modifiers::none(),
            })
        }
        WindowEvent::Resized(size) => EventKind::Resize(size.width as u16, size.height as u16),
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
    }

    #[test]
    fn test_event_creation() {
        let e = Event::key(Key::Enter);
        assert!(e.is_key(Key::Enter));
        assert!(!e.is_key(Key::Esc));
    }

    #[test]
    fn test_event_with_modifiers() {
        let mods = Modifiers {
            ctrl: true,
            ..Modifiers::none()
        };
        let e = Event::key_with_mods(Key::Char('c'), mods);
        assert!(e.is_key_with_mods(
            Key::Char('c'),
            Modifiers {
                ctrl: true,
                ..Modifiers::none()
            }
        ));
    }

    #[test]
    fn test_event_raw_none() {
        let e = Event::new(EventKind::FocusGained);
        assert!(e.raw.is_none());
    }

    #[test]
    fn test_modifiers_any() {
        assert!(!Modifiers::none().any());
        assert!(Modifiers {
            ctrl: true,
            ..Modifiers::none()
        }
        .any());
    }
}

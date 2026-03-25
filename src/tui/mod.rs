//! Terminal rendering backend using ANSI escape sequences
//!
//! Provides `TerminalRenderer` which implements the `Renderer` trait for
//! terminal applications using crossterm, with graphics support via
//! Kitty, Sixel, Unicode blocks, and Linux framebuffer backends.

mod renderer;

pub use renderer::TerminalRenderer;

//! mkui - A minimalist, typography-driven UI library targeting terminals and native GUIs
//!
//! A custom UI framework with:
//! - Trait-based rendering supporting terminal (ANSI) and GUI (wgpu) backends
//! - Native Kitty graphics protocol integration for terminals
//! - Immediate mode rendering with retained component structure
//! - Typography-first visual hierarchy
//! - Flex-based layout system
//! - Vim-like modal editing support

#![warn(missing_docs)]

pub mod app;
pub mod component;
pub mod components;
pub mod context;
pub mod event;
pub mod focus;
#[cfg(feature = "tui")]
pub mod graphics;
#[cfg(feature = "gui")]
pub mod gui;
pub mod i18n;
pub mod layout;
pub mod modal;
pub mod render;
pub mod slots;
pub mod style;
#[cfg(feature = "tui")]
pub mod terminal;
pub mod theme;
#[cfg(feature = "tui")]
pub mod tui;

// Re-export commonly used types
pub use app::App;
pub use component::Component;
pub use components::{
    Animation, CommandExecutor, CommandMode, CommandPalette, CommandResult, ConfirmPopup, Image,
    ImageData, List, Pane, Popup, PopupBorderStyle, PopupPosition, PopupResult, ScrollableView,
    SelectionMode, SplitDirection, SplitView, TextInput,
};
pub use context::{RenderContext, UseAccessibility, UseLocale, UseTheme};
pub use event::{Event, EventHandler, EventKind, Key, RawEvent};
#[cfg(feature = "tui")]
pub use event::{EventPoller, FrameTimer};
pub use focus::{ComponentId, FocusDirection, FocusManager, FocusableInfo};
#[cfg(feature = "tui")]
pub use graphics::GraphicsBackend;
pub use i18n::{AccessibilityRole, AccessibilitySettings, Locale, TextDirection};
pub use layout::Rect;
pub use modal::{
    KeyResult, ModalHandler, ModalState, Mode, Motion, Operator, SearchDirection, VisualMode,
};
pub use render::{DirtyRegion, ImageParams, Renderer};
#[cfg(feature = "tui")]
pub use tui::TerminalRenderer;
pub use slots::{header_slots, priority, status_slots, RegionSlots, SlotContent, Slots, UseSlots};
pub use style::{Selector, Style, StyleProperty, StyleRule, StyleSheet, Styleable};
#[cfg(feature = "tui")]
pub use terminal::{TerminalCapabilities, TerminalContext, TerminalGeometry, TmuxPaneInfo};
pub use theme::{BorderChars, BorderStyle, Color, Theme};

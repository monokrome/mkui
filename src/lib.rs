//! mkui - A minimalist, typography-driven TUI library with Kitty graphics support
//!
//! A custom UI framework optimized for audio/music applications with:
//! - Native Kitty graphics protocol integration
//! - Immediate mode rendering with retained component structure
//! - Typography-first visual hierarchy
//! - Flex-based layout system
//! - Vim-like modal editing support

pub mod component;
pub mod components;
pub mod context;
pub mod event;
pub mod focus;
pub mod graphics;
pub mod i18n;
pub mod layout;
pub mod modal;
pub mod render;
pub mod slots;
pub mod style;
pub mod terminal;
pub mod theme;

// Re-export commonly used types
pub use component::Component;
pub use components::{
    Animation, CommandExecutor, CommandMode, CommandPalette, CommandResult, ConfirmPopup, Image,
    ImageData, List, Pane, Popup, PopupBorderStyle, PopupPosition, PopupResult, ScrollableView,
    SelectionMode, SplitDirection, SplitView, TextInput,
};
pub use context::{RenderContext, UseAccessibility, UseLocale, UseTheme};
pub use event::{Event, EventHandler, Key};
pub use focus::{ComponentId, FocusDirection, FocusManager, FocusableInfo};
pub use graphics::GraphicsBackend;
pub use i18n::{AccessibilityRole, AccessibilitySettings, Locale, TextDirection};
pub use layout::Rect;
pub use modal::{
    KeyResult, ModalHandler, ModalState, Mode, Motion, Operator, SearchDirection, VisualMode,
};
pub use render::{DirtyRegion, Renderer};
pub use slots::{header_slots, priority, status_slots, RegionSlots, SlotContent, Slots, UseSlots};
pub use style::{Selector, Style, StyleProperty, StyleRule, StyleSheet, Styleable};
pub use terminal::{TerminalCapabilities, TerminalContext, TerminalGeometry, TmuxPaneInfo};
pub use theme::{BorderChars, BorderStyle, Color, Theme};

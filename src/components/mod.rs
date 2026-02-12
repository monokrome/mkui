//! Built-in UI components

pub mod command_palette;
pub mod container;
pub mod graphics_components;
pub mod header;
pub mod list;
pub mod logo;
pub mod popup;
pub mod scrollable;
pub mod slot_content;
pub mod slotted_bar;
pub mod split;
pub mod status_bar;
pub mod text;
pub mod text_input;
pub mod title;

pub use command_palette::{CommandExecutor, CommandMode, CommandPalette, CommandResult};
pub use container::Container;
pub use graphics_components::{Animation, Image, ImageData};
pub use header::Header;
pub use list::{List, SelectionMode};
pub use logo::Logo;
pub use popup::{ConfirmPopup, Popup, PopupBorderStyle, PopupPosition, PopupResult};
pub use scrollable::ScrollableView;
pub use slot_content::{Badge, Spacer, TextSlot};
pub use slotted_bar::{Slot, SlotContent, SlottedBar};
pub use split::{Pane, SplitDirection, SplitView};
pub use status_bar::StatusBar;
pub use text::Text;
pub use text_input::TextInput;
pub use title::Title;

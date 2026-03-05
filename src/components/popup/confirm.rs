//! Confirmation dialog builder

use super::{Popup, PopupBorderStyle};

/// Builder for confirmation dialogs
pub struct ConfirmPopup {
    popup: Popup,
}

impl ConfirmPopup {
    /// Creates a new confirmation dialog with the given message
    pub fn new(message: impl Into<String>) -> Self {
        let msg = message.into();
        let popup = Popup::message(format!("{}\n\n[Enter] Confirm  [Esc] Cancel", msg))
            .with_title("Confirm")
            .with_border(PopupBorderStyle::Rounded);

        Self { popup }
    }

    /// Overrides the default "Confirm" title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.popup = self.popup.with_title(title);
        self
    }

    /// Consumes the builder and returns the configured Popup
    pub fn build(self) -> Popup {
        self.popup
    }
}

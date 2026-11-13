//! Requests the window UI makes to its state machine.

use crate::menu::MenuAction;

/// A request the window UI makes to the state machine.
pub(crate) enum WindowRequest {
    /// A menu item was chosen.
    Menu(MenuAction),
    /// The user answered the unsaved-changes alert.
    Close(CloseChoice),
    /// The user answered the revert alert; `true` confirms.
    Revert(bool),
    /// The user cancelled a large close save.
    CancelSave,
}

/// The unsaved-changes alert choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseChoice {
    /// Save, then close.
    Save,
    /// Close and discard the edits.
    DontSave,
    /// Keep the window open.
    Cancel,
}

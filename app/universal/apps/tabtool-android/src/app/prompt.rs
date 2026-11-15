//! The banner, the unsaved-changes prompt and the back-gesture outcome.

use std::path::PathBuf;

/// A one-shot banner the app shows above the screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    /// A localized failure, by catalog key.
    Error(&'static str),
    /// A ready-made informational message.
    Message(String),
}

/// What the app should do after the prompt's save or discard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AfterSaveAction {
    /// Close the editor and return to the library.
    CloseEditor,
    /// Open this document.
    Open(PathBuf),
    /// Share the current document.
    Share,
}

/// The Save / Don't Save / Cancel prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavePrompt {
    /// What happens when the user saves or discards.
    pub action: AfterSaveAction,
    /// The document name the prompt shows.
    pub name: String,
}

/// The user's answer to the unsaved-changes prompt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptChoice {
    /// Write the document, then continue.
    Save,
    /// Discard the in-memory edits, then continue.
    DontSave,
    /// Stay where the user was.
    Cancel,
}

/// What one back gesture did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackResult {
    /// A sheet closed.
    ClosedSheet,
    /// The editor closed and the library is showing.
    ClosedDocument,
    /// A prompt is showing.
    Prompt,
    /// A commit is pending; the back runs after it lands.
    Deferred,
    /// The app kept the screen, for example while a save runs.
    Ignored,
    /// The prompt was cancelled.
    CancelledPrompt,
    /// The library is at the root, so the activity exits.
    Exit,
}

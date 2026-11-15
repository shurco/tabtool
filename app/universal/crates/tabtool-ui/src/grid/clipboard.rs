//! The clipboard seam the grid commands use.
//!
//! The UI crate must not reach for the platform clipboard itself: the app
//! supplies an implementation for egui's copy/paste events or for a test double.
//! Copy writes TSV; paste reads whatever text the platform delivered.

/// Text the grid copies to and pastes from.
pub trait Clipboard {
    /// Replaces the clipboard text.
    fn copy(&mut self, text: &str);

    /// The text available to paste, or `None` when the clipboard holds none.
    fn paste(&mut self) -> Option<String>;
}

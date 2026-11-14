//! Platform-neutral document, inset, share and back services.
//!
//! Mobile views depend on these traits only; the desktop and Android modules
//! own the implementations.

use std::path::{Path, PathBuf};

/// The extensions the app accepts and the open panel offers.
///
/// These match the Apple `Info.plist` document types and `DroppedFiles`.
pub const TABLE_EXTENSIONS: [&str; 5] = ["csv", "tsv", "txt", "tab", "dat"];

/// The table files in `directory`, sorted by path.
pub(crate) fn table_files(directory: &Path) -> Vec<PathBuf> {
    let mut documents: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_table_extension(path))
        .collect();
    documents.sort();
    documents
}

fn has_table_extension(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| {
            TABLE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// The app's private document store plus the user-facing file pickers.
///
/// Documents live in an app-private directory. Import copies the picked
/// document into that directory; export writes a copy to the destination the
/// user chooses, so the working file is never rebound to a user location.
pub trait DocumentFiles {
    /// The app's private documents directory.
    fn documents_dir(&self) -> PathBuf;

    /// The documents in [`DocumentFiles::documents_dir`] with a supported
    /// extension. The order is platform-defined; the library sorts for display.
    fn list_documents(&self) -> Vec<PathBuf>;

    /// Asks the user to pick a file and copies it into the documents
    /// directory, returning the copy.
    ///
    /// The result is one-shot. Desktop shows a modal panel and answers at once.
    /// Android's picker is asynchronous: the first call starts it and returns
    /// `None`; while the picker is open, calls return `None`; the copy is
    /// reported once by a later call. A cancel or a failed copy also returns
    /// `None`, so callers that need progress state use the platform's pending
    /// query.
    fn import_from_picker(&self) -> Option<PathBuf>;

    /// Writes a copy of `source` under `suggested_name` at a location the user
    /// chooses, and reports whether the copy completed.
    ///
    /// Cancel and failure are both `false`; as with
    /// [`DocumentFiles::import_from_picker`], Android answers over later calls.
    fn export_copy(&self, source: &Path, suggested_name: &str) -> bool;

    /// Deletes a document.
    fn delete(&self, path: &Path) -> std::io::Result<()>;

    /// Renames a document.
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
}

/// Window insets in points.
pub trait Insets {
    /// `[left, top, right, bottom]` safe-area insets.
    fn safe_area(&self) -> [f32; 4];

    /// The soft keyboard height, or zero when it is hidden.
    fn keyboard_height(&self) -> f32;
}

/// Hands a document to another app.
pub trait Share {
    /// Opens the system share sheet for `path`.
    fn share(&self, path: &Path);
}

/// The text clipboard.
pub trait Clipboard {
    /// The clipboard text, if there is any.
    fn get(&self) -> Option<String>;
    /// Replaces the clipboard text.
    fn set(&self, text: &str);
}

/// Opens a URL in the user's browser or mail client.
pub trait OpenUrl {
    /// Opens `url`.
    fn open(&self, url: &str);
}

/// Delivers the system back gesture to the app.
pub trait BackHandler {
    /// Takes a pending back event, if the system reported one.
    ///
    /// The platform consumes the system gesture and queues it; the app decides
    /// what to close and exits at the root itself.
    fn take_back(&self) -> bool;
}

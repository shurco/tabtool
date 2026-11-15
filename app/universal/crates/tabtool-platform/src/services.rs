//! Platform-neutral document, inset, share and back services.
//!
//! Mobile views depend on these traits only; the desktop and Android modules
//! own the implementations.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The extensions the app accepts and the open panel offers.
///
/// These match the Apple `Info.plist` document types and `DroppedFiles`.
/// `tests/kotlin_extensions.rs` keeps the Kotlin bridge's copy in sync.
pub const TABLE_EXTENSIONS: [&str; 5] = ["csv", "tsv", "txt", "tab", "dat"];

/// The catalog key for a platform failure with no more specific message.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) const ERROR_GENERIC: &str = "Something Went Wrong";

/// The catalog key for a document that could not be opened.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) const ERROR_OPEN: &str = "The file could not be opened";

/// The catalog key for a document that could not be written.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) const ERROR_SAVE: &str = "The file could not be saved";

/// A platform failure the UI can show as a localized banner.
///
/// The platform keeps the user-facing text out of the data: the app resolves
/// [`PlatformError::message_key`] through `tabtool-i18n`. `detail` is for
/// logcat only and is never shown raw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformError {
    /// The `tabtool-i18n` catalog key for the banner.
    pub message_key: &'static str,
    /// The technical cause, for logs.
    pub detail: String,
}

impl PlatformError {
    /// Builds a failure with a catalog key and a diagnostic detail.
    #[must_use]
    pub fn new(message_key: &'static str, detail: impl Into<String>) -> Self {
        Self {
            message_key,
            detail: detail.into(),
        }
    }
}

/// The platform's recent failures, so the app can show a localized banner.
pub trait PlatformErrors {
    /// Takes the next failure, if any.
    fn take_error(&self) -> Option<PlatformError>;
}

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

/// Resolves `path` once and caches only a success.
///
/// A `None` resolver result is retried on the next call, so a platform that is
/// still starting up cannot pin an empty path forever.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) fn cached_path(
    cache: &OnceLock<PathBuf>,
    resolve: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = cache.get() {
        return Some(path.clone());
    }
    let path = resolve()?;
    let _ = cache.set(path.clone());
    Some(path)
}

/// Refuses `path` unless its parent is `directory`.
///
/// This is a lexical guard for the app-owned store; it does not resolve
/// symlinks.
pub(crate) fn ensure_within(directory: &Path, path: &Path) -> io::Result<()> {
    if path.parent() == Some(directory) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is outside {}", path.display(), directory.display()),
        ))
    }
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

/// What an asynchronous export picker did.
///
/// The neutral [`DocumentFiles`] methods answer with `bool`; Android also
/// keeps the typed outcome so the UI can distinguish a cancel from a failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyOutcome {
    /// The copy was written.
    Copied,
    /// The user dismissed the picker.
    Cancelled,
    /// The copy failed; the cause is reported through [`PlatformErrors`].
    Failed,
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use super::{ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE, cached_path, ensure_within};

    #[test]
    fn a_failed_path_resolution_is_retried_and_a_success_is_cached() {
        let cache: OnceLock<PathBuf> = OnceLock::new();
        let attempts = Cell::new(0);

        let first = cached_path(&cache, || {
            attempts.set(attempts.get() + 1);
            None
        });
        assert_eq!(first, None);
        assert_eq!(attempts.get(), 1);

        let second = cached_path(&cache, || {
            attempts.set(attempts.get() + 1);
            Some(PathBuf::from("/data/files/documents"))
        });
        assert_eq!(second, Some(PathBuf::from("/data/files/documents")));
        assert_eq!(attempts.get(), 2);

        let third = cached_path(&cache, || {
            attempts.set(attempts.get() + 1);
            Some(PathBuf::from("/somewhere/else"))
        });
        assert_eq!(third, Some(PathBuf::from("/data/files/documents")));
        assert_eq!(attempts.get(), 2, "a cached path skips the resolver");
    }

    #[test]
    fn only_paths_directly_inside_the_directory_are_accepted() {
        let directory = Path::new("/data/files/documents");

        assert!(ensure_within(directory, &directory.join("a.csv")).is_ok());
        assert!(ensure_within(directory, Path::new("/sdcard/a.csv")).is_err());
        assert!(ensure_within(directory, &directory.join("nested/a.csv")).is_err());
        assert!(ensure_within(directory, Path::new("a.csv")).is_err());
    }

    #[test]
    fn every_platform_error_key_exists_in_the_catalog() {
        for key in [ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE] {
            assert!(
                tabtool_i18n::keys().any(|catalog_key| catalog_key == key),
                "{key} is missing from the catalog"
            );
        }
    }
}

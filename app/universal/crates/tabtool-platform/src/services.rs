//! Platform-neutral failures, export outcomes, clipboard and URL services,
//! plus the document-store helpers the Android module uses.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tabtool_core::has_table_extension;

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

/// The table files in `directory`, sorted by path.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
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
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
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

/// What an asynchronous export picker did.
///
/// The typed outcome lets the UI distinguish a cancel from a failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyOutcome {
    /// The copy was written.
    Copied,
    /// The user dismissed the picker.
    Cancelled,
    /// The copy failed; the cause is queued as a [`PlatformError`].
    Failed,
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use super::{cached_path, ensure_within};

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
}

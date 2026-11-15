//! The platform seam the library reads and writes documents through.

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tabtool_i18n::LocalizedError;

/// The metadata the library shows for a document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    /// The file size in bytes.
    pub size: u64,
    /// The last modification time.
    pub modified: SystemTime,
}

/// What a platform export picker did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportOutcome {
    /// The copy was written.
    Copied,
    /// The user cancelled the picker.
    Cancelled,
    /// The copy failed; the platform reports the cause separately.
    Failed,
}

/// The app's private document store, pickers, queued failures and file
/// metadata.
///
/// The app implements this seam over its platform, which keeps the UI crate
/// independent of the platform crate.
pub trait DocumentFiles {
    /// The app's private documents directory.
    fn documents_dir(&self) -> PathBuf;

    /// The table files in [`DocumentFiles::documents_dir`].
    fn list_documents(&self) -> Vec<PathBuf>;

    /// The names in the documents directory, including files the library
    /// does not list, for new-name collisions.
    fn existing_names(&self) -> Vec<String>;

    /// Writes a new document named `name` and returns its path.
    ///
    /// The platform creates the documents directory if needed and must not
    /// replace an existing file.
    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf>;

    /// Asks the user to pick a file and copies it into the documents
    /// directory, returning the copy.
    ///
    /// The result is one-shot: `None` means the picker is still open, the
    /// user cancelled, or the copy failed. Failures are reported separately
    /// through [`DocumentFiles::take_error`].
    fn import_from_picker(&self) -> Option<PathBuf>;

    /// Whether a picker started earlier is still open.
    ///
    /// A platform with a synchronous dialog answers `false`. The library
    /// waits while this is true and then takes the result with one last
    /// [`DocumentFiles::import_from_picker`] call.
    fn import_in_progress(&self) -> bool {
        false
    }

    /// Writes a copy of `source` under `suggested_name` at a location the user
    /// chooses.
    ///
    /// A platform with an asynchronous picker reports [`ExportOutcome::Cancelled`]
    /// while the picker is open and again when the user cancels; the finished
    /// outcome arrives on a later call, as with
    /// [`DocumentFiles::import_from_picker`].
    fn export_copy(&self, source: &Path, suggested_name: &str) -> ExportOutcome;

    /// Whether an export picker started earlier is still open.
    fn export_in_progress(&self) -> bool {
        false
    }

    /// Deletes a document.
    fn delete(&self, path: &Path) -> io::Result<()>;

    /// Renames a document.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Takes the platform's next queued failure, so the library can show a
    /// localized banner after an asynchronous import.
    ///
    /// A platform that reports failures directly answers `None`.
    fn take_error(&self) -> Option<LocalizedError> {
        None
    }

    /// Reads the metadata for `path`, or reports that it is gone.
    ///
    /// The default reads the live file system.
    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a regular file", path.display()),
            ));
        }
        Ok(DocumentMetadata {
            size: metadata.len(),
            // A file system without a modification date sorts as oldest.
            modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        })
    }
}

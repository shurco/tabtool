//! The app's documents directory and the import and export pickers.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tabtool_i18n::{ERROR_OPEN, ERROR_SAVE};

use super::jni_bridge::{call_string, call_string_2};
use super::{AndroidPlatform, record_error};
use crate::bridge_result::{BridgeResult, parse_result};
use crate::services::CopyOutcome;

static DOCUMENTS_DIR: OnceLock<PathBuf> = OnceLock::new();

impl AndroidPlatform {
    /// The app's private documents directory.
    pub fn documents_dir(self) -> PathBuf {
        crate::services::cached_path(&DOCUMENTS_DIR, || {
            call_string("documentsDir", "()Ljava/lang/String;", &[]).map(PathBuf::from)
        })
        .unwrap_or_default()
    }

    /// The documents in [`AndroidPlatform::documents_dir`] with a supported
    /// extension, sorted by path.
    pub fn list_documents(self) -> Vec<PathBuf> {
        crate::services::table_files(&self.documents_dir())
    }

    /// Asks the user to pick a file and copies it into the documents
    /// directory, returning the copy.
    ///
    /// The result is one-shot. The picker is asynchronous: the first call
    /// starts it and returns `None`; while the picker is open, calls return
    /// `None`; the copy is reported once by a later call. A cancel or a failed
    /// copy also returns `None`, so callers that need progress state use
    /// [`AndroidPlatform::import_pending`].
    pub fn import_from_picker(self) -> Option<PathBuf> {
        let raw = call_string("importDocument", "()Ljava/lang/String;", &[])?;
        match parse_result(&raw) {
            BridgeResult::Ok(Some(name)) => {
                let path = self.documents_dir().join(name);
                if path.is_file() {
                    Some(path)
                } else {
                    record_error(
                        ERROR_OPEN,
                        format!("the imported document is missing at {}", path.display()),
                    );
                    None
                }
            }
            BridgeResult::Ok(None) => None,
            BridgeResult::Cancelled => None,
            BridgeResult::Failed(message) => {
                record_error(ERROR_OPEN, format!("import: {message}"));
                None
            }
        }
    }

    /// Writes a copy of `source` under `suggested_name` at a location the user
    /// chooses.
    ///
    /// As with [`AndroidPlatform::import_from_picker`], the picker answers over
    /// later calls: `None` while it is open or when the bridge did not answer,
    /// then the typed outcome once.
    pub fn export_copy(self, source: &Path, suggested_name: &str) -> Option<CopyOutcome> {
        let source = source.to_string_lossy();
        let raw = call_string_2(
            "exportDocument",
            &source,
            suggested_name,
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        )?;
        Some(match parse_result(&raw) {
            BridgeResult::Ok(_) => CopyOutcome::Copied,
            BridgeResult::Cancelled => CopyOutcome::Cancelled,
            BridgeResult::Failed(message) => {
                record_error(ERROR_SAVE, format!("export: {message}"));
                CopyOutcome::Failed
            }
        })
    }

    /// Deletes a document.
    pub fn delete(self, path: &Path) -> io::Result<()> {
        crate::services::ensure_within(&self.documents_dir(), path)?;
        std::fs::remove_file(path)
    }

    /// Renames a document.
    pub fn rename(self, from: &Path, to: &Path) -> io::Result<()> {
        let directory = self.documents_dir();
        crate::services::ensure_within(&directory, from)?;
        crate::services::ensure_within(&directory, to)?;
        std::fs::rename(from, to)
    }
}

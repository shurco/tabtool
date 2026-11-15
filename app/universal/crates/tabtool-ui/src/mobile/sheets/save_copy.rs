//! The large-file Save As copy and the saved-copy notice.

use std::path::PathBuf;

use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use crate::mobile::{DocumentFiles, ExportOutcome};

/// The catalog key of the successful Save As notice.
pub const SAVED_COPY_KEY: &str = "Saved a copy to %@";

/// The Save As state for a large document.
///
/// The copy goes through [`DocumentFiles::export_copy`], so the working file
/// stays bound to the editor.
#[derive(Default)]
pub struct SaveCopy {
    pending: Option<PendingCopy>,
    result: Option<CopyResult>,
}

struct PendingCopy {
    source: PathBuf,
    name: String,
}

/// What Save As did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CopyResult {
    /// The copy was written under this file name.
    Saved(String),
    /// The user cancelled the picker.
    Cancelled,
    /// The copy failed.
    Failed,
}

impl SaveCopy {
    /// A Save As with no request yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the copy for a large document; returns whether one started.
    pub fn start(&mut self, document: &Document, files: &dyn DocumentFiles) -> bool {
        if self.pending.is_some() || !document.is_large_mode() {
            return false;
        }
        self.result = None;
        let Some(path) = document.file_path() else {
            return false;
        };
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            return false;
        };
        let outcome = files.export_copy(path, &name);
        if files.export_in_progress() {
            self.pending = Some(PendingCopy {
                source: path.to_path_buf(),
                name,
            });
        } else {
            self.result = Some(result(outcome, name));
        }
        true
    }

    /// Polls an asynchronous picker started by [`SaveCopy::start`].
    pub fn poll(&mut self, files: &dyn DocumentFiles) {
        if self.pending.is_none() || files.export_in_progress() {
            return;
        }
        let Some(pending) = self.pending.take() else {
            return;
        };
        let outcome = files.export_copy(&pending.source, &pending.name);
        self.result = Some(result(outcome, pending.name));
    }

    /// Whether the picker is still open.
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Takes the finished result, if any.
    pub fn take_result(&mut self) -> Option<CopyResult> {
        self.result.take()
    }
}

fn result(outcome: ExportOutcome, name: String) -> CopyResult {
    match outcome {
        ExportOutcome::Copied => CopyResult::Saved(name),
        ExportOutcome::Cancelled => CopyResult::Cancelled,
        ExportOutcome::Failed => CopyResult::Failed,
    }
}

/// The localized "Saved a copy to …" notice.
pub fn saved_copy_message(language: Language, name: &str) -> String {
    tr(language, SAVED_COPY_KEY).replace("%@", name)
}

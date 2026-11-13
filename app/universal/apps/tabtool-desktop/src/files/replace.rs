//! Replacing a document after an unsaved-changes decision.

use std::path::Path;

use tabtool_document::{Document, DocumentError, LargeStatus};
use tabtool_i18n::Language;

use super::{OpenedDocument, SaveOutcome};

use super::policy::FilePolicy;

/// How a replace request ended.
#[derive(Debug)]
pub enum ReplaceOutcome {
    /// The new document was loaded; a failed read is still opened.
    Opened(Box<OpenedDocument>),
    /// The current document is dirty; ask Save / Don't Save / Cancel.
    NeedsConfirmation,
    /// A large save owns the document or was started.
    Busy,
    /// The user cancelled the save or the replace.
    Cancelled,
    /// Saving the current document failed; nothing was replaced.
    Failed(DocumentError),
}

impl PartialEq for ReplaceOutcome {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NeedsConfirmation, Self::NeedsConfirmation)
            | (Self::Busy, Self::Busy)
            | (Self::Cancelled, Self::Cancelled) => true,
            (Self::Failed(left), Self::Failed(right)) => left == right,
            (Self::Opened(left), Self::Opened(right)) => {
                left.loaded == right.loaded
                    && left.document.file_path() == right.document.file_path()
            }
            _ => false,
        }
    }
}

impl Eq for ReplaceOutcome {}

/// The unsaved-change prompt choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsavedChoice {
    /// Save, then continue.
    Save,
    /// Continue and discard the edits.
    DontSave,
    /// Stop the operation.
    Cancel,
}

impl FilePolicy {
    /// Replaces `document` with `path` after the unsaved-change decision.
    ///
    /// `None` asks first for a dirty document and opens directly for a clean
    /// one. A running or newly started large save reports
    /// [`ReplaceOutcome::Busy`] so the window can show the progress and cancel
    /// it, or queue the path until the save settles.
    pub(crate) fn replace_with(
        &self,
        document: &mut Document,
        path: &Path,
        decision: Option<UnsavedChoice>,
        suggested_name: &str,
        language: Language,
    ) -> ReplaceOutcome {
        if matches!(document.large_status(), LargeStatus::Saving(_)) {
            return ReplaceOutcome::Busy;
        }
        let decision = match decision {
            Some(decision) => decision,
            None if document.is_dirty() => return ReplaceOutcome::NeedsConfirmation,
            None => UnsavedChoice::DontSave,
        };
        match decision {
            UnsavedChoice::Cancel => return ReplaceOutcome::Cancelled,
            UnsavedChoice::DontSave => {}
            UnsavedChoice::Save => match self.save(document, suggested_name) {
                SaveOutcome::Saved => {}
                SaveOutcome::LargeStarted => return ReplaceOutcome::Busy,
                SaveOutcome::Cancelled => return ReplaceOutcome::Cancelled,
                SaveOutcome::Failed(error) => return ReplaceOutcome::Failed(error),
            },
        }
        ReplaceOutcome::Opened(Box::new(self.load(path, language)))
    }
}

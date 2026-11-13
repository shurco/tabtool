//! Reverting a document to the file on disk.

use tabtool_document::{Document, DocumentError, RevertDecision, plan_revert};

use super::FileWorkflow;

/// How a revert request ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevertOutcome {
    /// The table was replaced with the saved content.
    Reverted,
    /// The document is dirty; ask before discarding.
    NeedsConfirmation,
    /// There is no file or content to revert.
    Unavailable,
    /// The re-read failed; the previous table is untouched.
    Failed(DocumentError),
}

impl FileWorkflow {
    /// Reverts the document to the file on disk.
    ///
    /// A dirty document requires `confirm`; otherwise the call only reports
    /// that the UI must ask first. The file is never written.
    pub fn revert(&mut self, document: &mut Document, confirm: bool) -> RevertOutcome {
        match plan_revert(document) {
            RevertDecision::Unavailable => RevertOutcome::Unavailable,
            RevertDecision::NeedsConfirmation if !confirm => RevertOutcome::NeedsConfirmation,
            RevertDecision::Revert | RevertDecision::NeedsConfirmation => {
                match document.revert_to_saved(&*self.storage) {
                    Ok(()) => RevertOutcome::Reverted,
                    Err(error) => RevertOutcome::Failed(error),
                }
            }
        }
    }
}

//! File > Revert to Saved: the confirmation decision and the re-read.
//!
//! The document owns the transactional re-read; this module answers whether
//! the UI must ask first and performs the revert after the answer. The file is
//! only read, never written.

use std::path::PathBuf;

use crate::Document;
use crate::DocumentPhase;
use crate::storage::Storage;
use crate::types::DocumentError;
use crate::undo::UndoAction;

/// What File > Revert to Saved should do for the current document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevertDecision {
    /// The document is clean; revert without asking.
    Revert,
    /// The document has unsaved changes; ask before reverting.
    NeedsConfirmation,
    /// There is no file or table to revert.
    Unavailable,
}

/// Decides whether a revert needs a confirmation dialog.
///
/// A missing path, a missing table and a document that is still loading are
/// unavailable. A clean file-backed document reverts directly; a dirty one
/// asks the user, exactly like `RevertCoordinator` in the Apple app.
pub fn plan_revert(document: &Document) -> RevertDecision {
    if document.file_path().is_none()
        || document.table().is_none()
        || matches!(document.phase(), DocumentPhase::Loading)
    {
        return RevertDecision::Unavailable;
    }
    if document.has_unsaved_changes() {
        RevertDecision::NeedsConfirmation
    } else {
        RevertDecision::Revert
    }
}

impl Document {
    /// Replaces the table with the file at the current path parsed with the
    /// current configuration.
    ///
    /// The caller asks the user first when [`plan_revert`] reports
    /// [`RevertDecision::NeedsConfirmation`]. The replacement is one
    /// [`UndoAction::RevertToSaved`] step and leaves the file untouched. A
    /// failed read or parse keeps the previous table and records no undo.
    pub fn revert_to_saved(&mut self, storage: &dyn Storage) -> Result<(), DocumentError> {
        if matches!(self.phase, DocumentPhase::Loading) || self.table.is_none() {
            return Err(DocumentError::NotReady);
        }
        let Some(path) = self.file_path().map(PathBuf::from) else {
            return Err(DocumentError::NotReady);
        };
        if !self.begin_reread() {
            return Err(DocumentError::RequiresConfirmation);
        }
        let configuration = self.configuration.clone();
        let result = self.read_and_apply(&path, configuration, storage, UndoAction::RevertToSaved);
        self.end_reread();
        result
    }
}

//! The storage, dialogs and settings shared by the app workflow and a window.

use std::path::Path;

use tabtool_i18n::Language;
use tabtool_platform::Settings;

use crate::platform::{SharedDialogs, SharedStorage};

use super::OpenedDocument;

/// The file services behind every document operation.
///
/// The app-level [`super::FileWorkflow`] adds the recent list and persistence;
/// a document window owns the same policy so save, revert, replace and export
/// run through one implementation.
pub(crate) struct FilePolicy {
    pub(crate) dialogs: SharedDialogs,
    pub(crate) storage: SharedStorage,
    pub(crate) settings: Settings,
}

impl FilePolicy {
    /// Creates a policy with the given services.
    ///
    /// The recent list stays with the workflow; this copy is never read.
    pub(crate) fn new(
        dialogs: SharedDialogs,
        storage: SharedStorage,
        mut settings: Settings,
    ) -> Self {
        settings.recent_documents.clear();
        Self {
            dialogs,
            storage,
            settings,
        }
    }

    /// Updates the settings; the recent list stays with the workflow.
    pub(crate) fn set_settings(&mut self, mut settings: Settings) {
        settings.recent_documents.clear();
        self.settings = settings;
    }

    /// Loads `path` with the current settings.
    pub(crate) fn load(&self, path: &Path, language: Language) -> OpenedDocument {
        super::load_document(path, &*self.storage, &self.settings, language)
    }
}

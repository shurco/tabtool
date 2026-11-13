//! File workflows: opening, recent documents, saving, reverting and replacing
//! a document, plus export sessions.
//!
//! The workflow owns the recent-document list and the settings it persists. It
//! uses the injected [`FileDialogs`] and [`Storage`], so it is testable without
//! a window and the app never calls `rfd` outside `DesktopPlatform`. Document
//! windows use the same [`FilePolicy`], so save, revert, replace and export
//! have one implementation.

mod export;
mod policy;
mod replace;
mod revert;
mod save;

pub use export::{ExportOutcome, ExportSession, export_session, run_export};
pub(crate) use policy::FilePolicy;
pub use replace::{ReplaceOutcome, UnsavedChoice};
pub use revert::RevertOutcome;
pub use save::{SaveMode, SaveOutcome, save_document};

use std::path::{Path, PathBuf};

use tabtool_document::{Document, DocumentPhase};
use tabtool_i18n::Language;
pub use tabtool_platform::TABLE_EXTENSIONS;
use tabtool_platform::{RECENT_DOCUMENTS_LIMIT, Settings, save_settings};

use crate::platform::{SharedDialogs, SharedStorage};

/// Whether `path` has a table extension.
pub fn is_table_file(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|extension| TABLE_EXTENSIONS.contains(&extension.as_str()))
}

/// The first launch argument that names a file.
///
/// A leading `--flag` is skipped; the first argument without a leading dash is
/// the document to open.
pub fn launch_path(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    args.into_iter()
        .find(|argument| !argument.is_empty() && !argument.starts_with('-'))
        .map(PathBuf::from)
}

/// Loads `path` into a new document with `settings`.
///
/// A failed read still returns its document with the error phase, so both the
/// app and a window can show the read-error overlay. The caller decides
/// whether the path joins the recent list.
pub(crate) fn load_document(
    path: &Path,
    storage: &dyn tabtool_document::Storage,
    settings: &Settings,
    language: Language,
) -> OpenedDocument {
    let configuration = settings.new_document_configuration();
    let mut document = Document::new(configuration);
    let _ = document.open_file(
        path,
        storage,
        None,
        matches!(language, Language::ZhHans),
        settings.large_file_threshold_bytes(),
    );
    let loaded = matches!(
        document.phase(),
        DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
    ) && document.has_content();
    OpenedDocument { document, loaded }
}

/// A document opened by the workflow.
pub struct OpenedDocument {
    /// The document, including a failed read with its error phase.
    pub document: Document,
    /// Whether the read succeeded and the content is ready.
    pub loaded: bool,
}

impl core::fmt::Debug for OpenedDocument {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("OpenedDocument")
            .field("loaded", &self.loaded)
            .field("path", &self.document.file_path())
            .field("phase", self.document.phase())
            .finish()
    }
}

/// The recent-document list and the settings it persists.
pub struct FileWorkflow {
    policy: FilePolicy,
    settings_path: Option<PathBuf>,
    language: Language,
    /// The canonical recent list; the settings mirror is rebuilt from it.
    recent: Vec<PathBuf>,
}

impl FileWorkflow {
    /// Creates the workflow for `settings`.
    ///
    /// `settings_path` is `None` in tests, so no preference file is touched.
    pub fn new(
        dialogs: SharedDialogs,
        storage: SharedStorage,
        settings: Settings,
        settings_path: Option<PathBuf>,
    ) -> Self {
        let language = match settings.app_language {
            Language::System => Language::En,
            language => language,
        };
        let mut settings = settings;
        let recent = std::mem::take(&mut settings.recent_documents);
        let mut workflow = Self {
            policy: FilePolicy::new(dialogs, storage, settings),
            settings_path,
            language,
            recent,
        };
        workflow.prune_recent();
        workflow
    }

    /// Replaces the non-recent settings and drops disappeared recent paths.
    ///
    /// The recent list belongs to the workflow; the incoming settings mirror
    /// cannot overwrite it.
    pub fn set_settings(&mut self, settings: Settings) {
        self.policy.set_settings(settings);
        self.prune_recent();
    }

    /// Updates the interface language used for format detection.
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// The existing recent documents, newest first.
    pub fn recent(&self) -> &[PathBuf] {
        &self.recent
    }

    /// Whether any recent document remains.
    pub fn has_recent(&self) -> bool {
        !self.recent.is_empty()
    }

    /// Drops recent paths that no longer exist and persists the change.
    pub fn forget_missing_recent(&mut self) {
        let before = self.recent.len();
        self.prune_recent();
        if self.recent.len() != before {
            self.persist_recent();
        }
    }

    /// A blank untitled document in the New Document format.
    pub fn new_document(&self) -> Document {
        let configuration = self.policy.settings.new_document_configuration();
        let mut document = Document::new(configuration.clone());
        document
            .new_document(configuration)
            .expect("a new untitled document is never busy");
        document
    }

    /// Opens `path` through the size router; a failed read still returns the
    /// document so the window can show the read-error overlay.
    pub fn open_path(&mut self, path: &Path) -> OpenedDocument {
        let opened = self.policy.load(path, self.language);
        if opened.loaded {
            self.remember(path);
        }
        opened
    }

    /// Shows the open panel and loads every table file it returns.
    pub fn open_dialog(&mut self) -> Vec<OpenedDocument> {
        let paths = self.policy.dialogs.open_files();
        self.open_paths(&paths)
    }

    /// Loads the table files from a drop and ignores every other file.
    pub fn open_dropped(&mut self, paths: &[PathBuf]) -> Vec<OpenedDocument> {
        self.open_paths(paths)
    }

    /// Opens the recent document at `index`.
    ///
    /// A path deleted since it was remembered is dropped from the list and the
    /// call returns `None`.
    pub fn open_recent(&mut self, index: usize) -> Option<OpenedDocument> {
        let path = self.recent.get(index).cloned()?;
        if !path.exists() {
            self.recent.remove(index);
            self.persist_recent();
            return None;
        }
        Some(self.open_path(&path))
    }

    /// Adds `path` to the recent documents, newest first and at most ten.
    pub fn remember(&mut self, path: &Path) {
        self.prune_recent();
        self.recent.retain(|recent| recent != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT_DOCUMENTS_LIMIT);
        self.persist_recent();
    }

    /// Saves the document in place, or through the save panel when it has no
    /// path yet.
    pub fn save(&self, document: &mut Document, suggested_name: &str) -> SaveOutcome {
        self.policy.save(document, suggested_name)
    }

    /// Saves the document through the save panel.
    pub fn save_as(&self, document: &mut Document, suggested_name: &str) -> SaveOutcome {
        self.policy.save_as(document, suggested_name)
    }

    /// Reverts the document to the file on disk.
    ///
    /// A dirty document requires `confirm`; otherwise the call only reports
    /// that the UI must ask first. The file is never written.
    pub fn revert(&self, document: &mut Document, confirm: bool) -> RevertOutcome {
        self.policy.revert(document, confirm)
    }

    /// Replaces `document` with `path` after the unsaved-change decision.
    ///
    /// `None` asks first for a dirty document and opens directly for a clean
    /// one. A running or newly started large save reports
    /// [`ReplaceOutcome::Busy`] so the window can show the progress and cancel
    /// it.
    pub fn replace_with(
        &mut self,
        document: &mut Document,
        path: &Path,
        decision: Option<UnsavedChoice>,
        suggested_name: &str,
    ) -> ReplaceOutcome {
        let outcome =
            self.policy
                .replace_with(document, path, decision, suggested_name, self.language);
        let opened_path = match &outcome {
            ReplaceOutcome::Opened(opened) if opened.loaded => {
                opened.document.file_path().map(Path::to_path_buf)
            }
            _ => None,
        };
        if let Some(path) = opened_path {
            self.remember(&path);
        }
        outcome
    }

    /// Captures the visible table and format for an export panel.
    pub fn export_session(&self, document: &Document) -> Option<ExportSession> {
        self.policy.export_session(document)
    }

    /// Runs an export panel's candidate through preflight and the save panel.
    pub fn export(&self, session: &ExportSession, allow_ambiguous: bool) -> ExportOutcome {
        self.policy.export(session, allow_ambiguous)
    }

    fn open_paths(&mut self, paths: &[PathBuf]) -> Vec<OpenedDocument> {
        paths
            .iter()
            .filter(|path| is_table_file(path))
            .map(|path| self.open_path(path))
            .collect()
    }

    fn prune_recent(&mut self) {
        self.recent.retain(|path| path.exists());
    }

    fn persist_recent(&self) {
        if let Some(path) = &self.settings_path {
            let mut settings = self.policy.settings.clone();
            settings.recent_documents = self.recent.clone();
            let _ = save_settings(&settings, path);
        }
    }
}

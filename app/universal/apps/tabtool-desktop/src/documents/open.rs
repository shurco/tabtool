//! Opening another file in an existing window: drops, prompts and replacement.

use std::path::PathBuf;

use tabtool_ui::document::DocumentUiState;
use tabtool_ui::grid::GridState;

use crate::autosave::Autosave;
use crate::documents::{
    CloseChoice, CloseDecision, CloseSave, DocumentWindow, WindowRequest, WindowState,
    close_decision,
};
use crate::files::{is_table_file, load_document};

impl DocumentWindow {
    /// Accepts table files dropped on this window.
    ///
    /// The first file replaces this document after the unsaved prompt; the
    /// rest are returned by [`DocumentWindow::take_dropped_files`] so the app
    /// can open them as new documents.
    pub fn drop_files(&self, paths: Vec<PathBuf>) {
        self.lock().accept_dropped(paths);
    }

    /// Takes the extra dropped files the app should open as new documents.
    pub fn take_dropped_files(&self) -> Vec<PathBuf> {
        std::mem::take(&mut self.lock().open.dropped)
    }

    /// Takes the paths this window opened successfully for the recent list.
    pub fn take_opened_paths(&self) -> Vec<PathBuf> {
        std::mem::take(&mut self.lock().open.opened_paths)
    }

    /// Whether the open-another unsaved-changes alert is showing.
    pub fn has_open_prompt(&self) -> bool {
        self.lock().open.prompt
    }

    /// Answers the open-another alert; the next tick performs the choice.
    pub fn answer_open_prompt(&self, choice: CloseChoice) {
        self.lock().requests.push(WindowRequest::Open(choice));
    }

    /// Whether a busy modal is showing, e.g. a large save during a drop.
    pub fn is_busy(&self) -> bool {
        self.lock().busy
    }
}

/// The open-another state of one window.
#[derive(Default)]
pub(crate) struct OpenState {
    /// The unsaved-changes alert is showing for `path`.
    pub(crate) prompt: bool,
    /// The file waiting for the prompt answer.
    pub(crate) path: Option<PathBuf>,
    /// A large save must finish before this file opens.
    pub(crate) after_save: Option<PathBuf>,
    /// Successfully opened paths the app should add to Recent.
    pub(crate) opened_paths: Vec<PathBuf>,
    /// Extra dropped files the app opens as new documents.
    pub(crate) dropped: Vec<PathBuf>,
}

impl WindowState {
    /// Handles table files dropped on this window.
    pub(crate) fn accept_dropped(&mut self, paths: Vec<PathBuf>) {
        if self.open.prompt {
            return;
        }
        let mut files: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| is_table_file(path))
            .collect();
        if files.is_empty() {
            return;
        }
        let first = files.remove(0);
        self.open.dropped.extend(files);
        self.request_open(first);
    }

    /// Opens `path` now, or asks first when the document is dirty.
    pub(crate) fn request_open(&mut self, path: PathBuf) {
        match close_decision(&self.document) {
            CloseDecision::Close => self.begin_open(path),
            CloseDecision::Ask => {
                self.open.prompt = true;
                self.open.path = Some(path);
            }
            CloseDecision::Busy => self.busy = true,
        }
    }

    /// Replaces this window's document with the file at `path`.
    ///
    /// A failed read still replaces the document, so the error overlay explains
    /// the failure. Only a successful read enters the recent list.
    pub(crate) fn begin_open(&mut self, path: PathBuf) {
        let opened = load_document(&path, &*self.storage, &self.settings, self.language);
        if opened.loaded {
            self.open.opened_paths.push(path);
        }

        self.document = opened.document;
        self.grid = GridState::default();
        self.chrome = DocumentUiState::default();
        self.export = super::export::ExportState::default();
        self.pending_actions.clear();
        self.autosave = Autosave::new(&self.document);
        self.issue = None;
        self.close_prompt = false;
        self.revert_prompt = false;
        self.open.prompt = false;
        self.open.path = None;
        self.open.after_save = None;
        self.saving = false;
        self.close_after_save = false;
        self.busy = false;
    }

    /// Saves the current document, then opens the pending file.
    ///
    /// A large save reports Busy and defers the open until the save settles;
    /// cancelling or failing the save aborts the replace.
    pub(crate) fn save_then_open(&mut self) {
        let Some(path) = self.open.path.clone() else {
            return;
        };
        self.open.prompt = false;
        match self.save_auto() {
            CloseSave::Saved => self.begin_open(path),
            CloseSave::Saving => self.open.after_save = Some(path),
            CloseSave::Cancelled => self.open.path = None,
            CloseSave::Failed(error) => {
                self.issue = Some(error);
                self.open.path = None;
            }
        }
    }

    /// Runs an answer to the open-another alert.
    pub(crate) fn answer_open(&mut self, choice: CloseChoice) {
        self.open.prompt = false;
        match choice {
            CloseChoice::Save => self.save_then_open(),
            CloseChoice::DontSave => {
                if let Some(path) = self.open.path.take() {
                    self.begin_open(path);
                }
            }
            CloseChoice::Cancel => self.open.path = None,
        }
    }
}

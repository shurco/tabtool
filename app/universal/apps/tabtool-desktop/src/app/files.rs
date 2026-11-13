//! App-level open commands: dialog, path, launch argument and recent.

use std::path::{Path, PathBuf};

use crate::app::DesktopApp;
use crate::files::OpenedDocument;

impl DesktopApp {
    /// Shows the open panel and adds every selected table file.
    pub fn open_from_dialog(&mut self) {
        let opened = self.files.open_dialog();
        self.push_opened(opened);
    }

    /// Opens dropped table files as new document windows.
    pub fn open_dropped(&mut self, paths: &[PathBuf]) {
        let opened = self.files.open_dropped(paths);
        self.push_opened(opened);
    }

    /// Opens one path as a new document window.
    ///
    /// Returns whether the read succeeded; a failed read still opens the
    /// window so its error overlay can explain the failure.
    pub fn open_path(&mut self, path: &Path) -> bool {
        let opened = self.files.open_path(path);
        let loaded = opened.loaded;
        self.push_opened(vec![opened]);
        loaded
    }

    /// Opens the recent document at `index` as a new window.
    pub fn open_recent(&mut self, index: usize) -> bool {
        let Some(opened) = self.files.open_recent(index) else {
            self.sync_recent();
            return false;
        };
        let loaded = opened.loaded;
        self.push_opened(vec![opened]);
        loaded
    }

    /// Copies the workflow's recent list into the app settings mirror.
    pub(crate) fn sync_recent(&mut self) {
        self.settings.recent_documents = self.files.recent().to_vec();
    }

    fn push_opened(&mut self, opened: Vec<OpenedDocument>) {
        for opened in opened {
            self.add_document(opened.document);
        }
        self.sync_recent();
    }
}

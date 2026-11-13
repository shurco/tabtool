//! Saving for close and quit.

use std::path::Path;

use tabtool_document::DocumentError;
use tabtool_platform::FileDialogs;

use crate::documents::{CloseSave, WindowState};

impl WindowState {
    /// Saves the document for a close request.
    pub(crate) fn save_for_close(&mut self) -> CloseSave {
        self.save(true, false)
    }

    /// Saves the document through File > Save.
    pub(crate) fn save_auto(&mut self) -> CloseSave {
        self.save(false, false)
    }

    /// Saves the document through File > Save As.
    pub(crate) fn save_with_dialog(&mut self) -> CloseSave {
        self.save(false, true)
    }

    /// Starts or performs the save.
    ///
    /// `close_after` closes the window when a large job settles; the small
    /// path is synchronous and the caller acts on [`CloseSave::Saved`].
    fn save(&mut self, close_after: bool, force_dialog: bool) -> CloseSave {
        if force_dialog || self.document.file_path().is_none() {
            return self.save_as_dialog(close_after);
        }
        if self.document.is_large_mode() {
            return self.start_large_save(None, close_after);
        }
        match self.document.save(&*self.storage) {
            Ok(()) => {
                self.issue = None;
                CloseSave::Saved
            }
            Err(error) => {
                self.issue = Some(error);
                CloseSave::Failed(error)
            }
        }
    }

    fn save_as_dialog(&mut self, close_after: bool) -> CloseSave {
        let suggested = self.suggested_file_name();
        let Some(path) = self.platform.save_file(&suggested, "csv") else {
            return CloseSave::Cancelled;
        };
        if self.document.is_large_mode() {
            return self.start_large_save(Some(&path), close_after);
        }
        match self.document.save_as(&path, &*self.storage) {
            Ok(()) => {
                self.issue = None;
                CloseSave::Saved
            }
            Err(error) => {
                self.issue = Some(error);
                CloseSave::Failed(error)
            }
        }
    }

    fn start_large_save(&mut self, destination: Option<&Path>, close_after: bool) -> CloseSave {
        match self.document.start_large_save(destination) {
            Ok(_) => {
                self.saving = true;
                self.close_after_save = close_after;
                CloseSave::Saving
            }
            Err(_) => {
                let error = DocumentError::CannotWriteFile;
                self.issue = Some(error);
                CloseSave::Failed(error)
            }
        }
    }

    /// Polls a large save. Returns `None` while it is still writing.
    pub(crate) fn poll_save(&mut self) -> Option<Result<(), DocumentError>> {
        if !self.saving {
            return None;
        }
        if self
            .document
            .large_save_progress()
            .is_some_and(|progress| progress < 1.0)
        {
            return None;
        }

        let result = self
            .document
            .wait_for_large_save()
            .map_err(|_| DocumentError::CannotWriteFile);
        self.saving = false;
        if let Err(error) = result {
            self.issue = Some(error);
        }
        if result.is_ok() && self.close_after_save {
            self.close_after_save = false;
            self.should_close = true;
        }
        Some(result)
    }

    /// Cancels a large save and aborts the close.
    pub(crate) fn cancel_save(&mut self) {
        self.document.cancel_large_save();
        let _ = self.document.wait_for_large_save();
        self.saving = false;
        self.close_after_save = false;
        self.busy = false;
    }

    /// The name the save panel starts with.
    fn suggested_file_name(&self) -> String {
        let base = self
            .document
            .file_path()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.title());
        if base.to_ascii_lowercase().ends_with(".csv") {
            base
        } else {
            format!("{base}.csv")
        }
    }
}

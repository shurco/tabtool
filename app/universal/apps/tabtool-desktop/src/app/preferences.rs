//! Merging the Settings window into the app and persisting the result.

use tabtool_platform::{OpenUrl, save_settings};

use crate::about_window::{AboutAction, REVIEW_URL, WEBSITE_URL};

use super::DesktopApp;

impl DesktopApp {
    /// Takes the settings the Settings form edited into the app.
    pub(super) fn sync_settings_window(&mut self) {
        let Some(window) = &self.settings_window_state else {
            return;
        };
        if let Some(mut candidate) = window.take_changed() {
            // The recent list belongs to the file workflow; the form never edits it.
            candidate.recent_documents = self.settings.recent_documents.clone();
            if candidate != self.settings {
                self.settings = candidate;
                self.settings_dirty = true;
            }
        }
        window.set_settings(self.settings.clone());
    }

    /// Writes the settings after a change; the app owns the path, the form
    /// only edits the values.
    pub(super) fn persist_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        self.settings_dirty = false;
        if let Some(path) = &self.settings_path {
            let _ = save_settings(&self.settings, path);
        }
    }

    /// Opens the link the About window asked for.
    pub(super) fn take_about_action(&mut self) {
        let action = self
            .about_action
            .lock()
            .ok()
            .and_then(|mut action| action.take());
        match action {
            Some(AboutAction::Website) => self.platform.open(WEBSITE_URL),
            Some(AboutAction::Rate) => self.platform.open(REVIEW_URL),
            None => {}
        }
    }
}

//! Modal alerts for unsaved changes, revert, close saves and busy documents.

use egui::Ui;
use tabtool_i18n::tr;
use tabtool_ui::widgets::{AlertAction, ButtonKind, modal};

use crate::documents::{CloseChoice, WindowRequest, WindowState};

impl WindowState {
    pub(crate) fn render_alerts(&mut self, ui: &mut Ui) {
        if self.close_prompt {
            self.render_close_prompt(ui);
        }
        if self.revert_prompt {
            self.render_revert_prompt(ui);
        }
        if self.open.prompt {
            self.render_open_prompt(ui);
        }
        if self.saving && (self.close_after_save || self.open.after_save.is_some()) {
            self.render_close_saving(ui);
        } else if self.busy {
            self.render_busy(ui);
        }
    }

    fn render_open_prompt(&mut self, ui: &mut Ui) {
        let title = self.title();
        let actions = [
            AlertAction::new(tr(self.language, "Save"), ButtonKind::Prominent),
            AlertAction::new(tr(self.language, "Don’t Save"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let chosen = modal(
            ui,
            ("open-prompt", title.clone()),
            &title,
            tr(
                self.language,
                "Your changes will be lost if you don’t save them.",
            ),
            &actions,
        );
        let Some(index) = chosen else {
            return;
        };
        self.open.prompt = false;
        let choice = match index {
            0 => CloseChoice::Save,
            1 => CloseChoice::DontSave,
            _ => CloseChoice::Cancel,
        };
        self.requests
            .push(crate::documents::WindowRequest::Open(choice));
    }

    fn render_close_prompt(&mut self, ui: &mut Ui) {
        let title = self.title();
        let actions = [
            AlertAction::new(tr(self.language, "Save"), ButtonKind::Prominent),
            AlertAction::new(tr(self.language, "Don’t Save"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let chosen = modal(
            ui,
            ("close-prompt", title.clone()),
            &title,
            tr(
                self.language,
                "Your changes will be lost if you don’t save them.",
            ),
            &actions,
        );
        let Some(index) = chosen else {
            return;
        };
        self.close_prompt = false;
        let choice = match index {
            0 => CloseChoice::Save,
            1 => CloseChoice::DontSave,
            _ => CloseChoice::Cancel,
        };
        self.requests.push(WindowRequest::Close(choice));
    }

    fn render_revert_prompt(&mut self, ui: &mut Ui) {
        let actions = [
            AlertAction::new(tr(self.language, "Revert"), ButtonKind::Destructive),
            AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
        ];
        let chosen = modal(
            ui,
            "revert-prompt",
            tr(self.language, "Revert to Saved?"),
            tr(
                self.language,
                "Your unsaved changes will be discarded. The table will be replaced with the last saved version from disk.",
            ),
            &actions,
        );
        let Some(index) = chosen else {
            return;
        };
        self.revert_prompt = false;
        self.requests.push(WindowRequest::Revert(index == 0));
    }

    fn render_close_saving(&mut self, ui: &mut Ui) {
        let progress = self.document.large_save_progress().unwrap_or(0.0);
        let actions = [AlertAction::new(
            tr(self.language, "Cancel"),
            ButtonKind::Bordered,
        )];
        if modal(
            ui,
            "close-saving",
            &crate::documents::saving_text(self.language, progress),
            "",
            &actions,
        )
        .is_some()
        {
            self.requests.push(WindowRequest::CancelSave);
        }
    }

    fn render_busy(&mut self, ui: &mut Ui) {
        let progress = self.document.large_save_progress().unwrap_or(0.0);
        let actions = [AlertAction::new(
            tr(self.language, "OK"),
            ButtonKind::Bordered,
        )];
        if modal(
            ui,
            "close-busy",
            &crate::documents::saving_text(self.language, progress),
            "",
            &actions,
        )
        .is_some()
        {
            self.busy = false;
        }
    }
}

//! Quit policy: aggregate close decisions and save every dirty document.

use egui::{Ui, ViewportCommand, ViewportId};
use tabtool_document::DocumentError;
use tabtool_i18n::tr;
use tabtool_ui::widgets::{AlertAction, ButtonKind, modal};

use crate::app::DesktopApp;
use crate::documents::{CloseSave, DocumentWindow};

/// What quitting should do now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuitDecision {
    /// Every document is clean; quit immediately.
    Quit,
    /// At least one document is dirty; ask Save / Don't Save / Cancel.
    Ask,
    /// A save owns a document; do not quit yet.
    Busy,
}

/// Aggregates the close decisions of every open document.
pub fn quit_decision(documents: &[DocumentWindow]) -> QuitDecision {
    let mut ask = false;
    for window in documents {
        match window.close_decision() {
            crate::documents::CloseDecision::Busy => return QuitDecision::Busy,
            crate::documents::CloseDecision::Ask => ask = true,
            crate::documents::CloseDecision::Close => {}
        }
    }
    if ask {
        QuitDecision::Ask
    } else {
        QuitDecision::Quit
    }
}

/// The in-progress quit state.
#[derive(Default)]
pub(crate) struct QuitState {
    pub(crate) prompt: bool,
    pub(crate) busy: bool,
    pub(crate) issue: Option<DocumentError>,
    pub(crate) saving: Vec<ViewportId>,
}

impl DesktopApp {
    /// Handles File > Quit and Cmd+Q.
    pub(crate) fn request_quit(&mut self, ctx: &egui::Context) {
        if self.quit.prompt || self.quit.busy || self.quit.issue.is_some() {
            return;
        }
        match quit_decision(&self.documents) {
            QuitDecision::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            QuitDecision::Ask => self.quit.prompt = true,
            QuitDecision::Busy => self.quit.busy = true,
        }
    }

    /// Saves every dirty document, then closes the app.
    fn confirm_quit(&mut self, ctx: &egui::Context) {
        self.quit.prompt = false;
        self.quit.issue = None;
        self.quit.saving.clear();

        let windows: Vec<DocumentWindow> = self.documents.to_vec();
        for window in windows {
            if !window.is_dirty() {
                continue;
            }
            match window.begin_close_save() {
                CloseSave::Saved => {}
                CloseSave::Saving => self.quit.saving.push(window.viewport_id()),
                CloseSave::Cancelled => {
                    self.quit.saving.clear();
                    return;
                }
                CloseSave::Failed(error) => {
                    self.quit.saving.clear();
                    self.quit.issue = Some(error);
                    return;
                }
            }
        }

        if self.quit.saving.is_empty() {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Polls large quit saves and closes when they finish.
    pub(crate) fn poll_quit(&mut self, ctx: &egui::Context) {
        if self.quit.saving.is_empty() {
            return;
        }

        let mut remaining = Vec::new();
        let mut failed = None;
        for viewport_id in std::mem::take(&mut self.quit.saving) {
            let Some(window) = self
                .documents
                .iter()
                .find(|window| window.viewport_id() == viewport_id)
            else {
                continue;
            };
            match window.poll_close_save() {
                None => {
                    if window.is_saving() {
                        remaining.push(viewport_id);
                    }
                }
                Some(Ok(())) => {}
                Some(Err(error)) => failed = Some(error),
            }
        }
        self.quit.saving = remaining;

        if let Some(error) = failed {
            self.quit.issue = Some(error);
            return;
        }
        if self.quit.saving.is_empty() {
            if self.documents.iter().all(|window| !window.is_dirty()) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                self.quit.issue = Some(DocumentError::CannotWriteFile);
            }
        }
    }

    /// Renders the quit alert, the busy alert and a failed quit.
    pub(crate) fn render_quit_alerts(&mut self, ui: &mut Ui) {
        if self.quit.prompt {
            let actions = [
                AlertAction::new(tr(self.language, "Save"), ButtonKind::Prominent),
                AlertAction::new(tr(self.language, "Don’t Save"), ButtonKind::Destructive),
                AlertAction::new(tr(self.language, "Cancel"), ButtonKind::Bordered),
            ];
            if let Some(index) = modal(
                ui,
                "quit-prompt",
                tr(self.language, "Quit"),
                tr(
                    self.language,
                    "Your changes will be lost if you don’t save them.",
                ),
                &actions,
            ) {
                self.quit.prompt = false;
                match index {
                    0 => self.confirm_quit(ui.ctx()),
                    1 => ui.ctx().send_viewport_cmd(ViewportCommand::Close),
                    _ => {}
                }
            }
        }

        if self.quit.busy {
            let actions = [AlertAction::new(
                tr(self.language, "OK"),
                ButtonKind::Bordered,
            )];
            let progress = self
                .documents
                .iter()
                .filter_map(|window| window.save_progress())
                .reduce(f64::max)
                .unwrap_or(0.0);
            if modal(
                ui,
                "quit-busy",
                &crate::documents::saving_text(self.language, progress),
                "",
                &actions,
            )
            .is_some()
            {
                self.quit.busy = false;
            }
        }

        if let Some(error) = self.quit.issue {
            let actions = [AlertAction::new(
                tr(self.language, "OK"),
                ButtonKind::Bordered,
            )];
            if modal(
                ui,
                "quit-issue",
                tr(self.language, "The file could not be saved"),
                &crate::documents::issue_message(self.language, error),
                &actions,
            )
            .is_some()
            {
                self.quit.issue = None;
            }
        }
    }
}

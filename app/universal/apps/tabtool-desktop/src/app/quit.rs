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

/// How the quit alert was answered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuitResponse {
    /// Save every dirty document, then quit.
    Save,
    /// Quit and discard the edits.
    DontSave,
    /// Stay open.
    Cancel,
}

/// The in-progress quit state.
#[derive(Default)]
pub(crate) struct QuitState {
    pub(crate) prompt: bool,
    pub(crate) busy: bool,
    pub(crate) issue: Option<DocumentError>,
    pub(crate) saving: Vec<ViewportId>,
    /// The user confirmed the quit; stop re-prompting on the root close.
    pub(crate) force_close: bool,
}

impl DesktopApp {
    /// Handles File > Quit and Cmd+Q.
    pub fn request_quit(&mut self, ctx: &egui::Context) {
        if self.quit.force_close || self.quit.prompt || self.quit.busy || self.quit.issue.is_some()
        {
            return;
        }
        match quit_decision(&self.documents) {
            QuitDecision::Quit => {
                self.quit.force_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            QuitDecision::Ask => self.quit.prompt = true,
            QuitDecision::Busy => self.quit.busy = true,
        }
    }

    /// Whether the app has decided to quit and must not ask again.
    pub fn is_quitting(&self) -> bool {
        self.quit.force_close
    }

    /// Whether the quit alert is showing.
    pub fn is_quit_prompt_visible(&self) -> bool {
        self.quit.prompt
    }

    /// Answers the quit alert; `Save` may start large saves.
    pub fn answer_quit_prompt(&mut self, response: QuitResponse, ctx: &egui::Context) {
        self.quit.prompt = false;
        match response {
            QuitResponse::Save => self.confirm_quit(ctx),
            QuitResponse::DontSave => {
                self.quit.force_close = true;
                // A save left over from an earlier attempt must not keep a
                // mapping alive after the app exits.
                for window in &self.documents {
                    window.cancel_close_save();
                }
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            QuitResponse::Cancel => {}
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
                    self.abort_quit();
                    return;
                }
                CloseSave::Failed(error) => {
                    self.abort_quit();
                    self.quit.issue = Some(error);
                    return;
                }
            }
        }

        if self.quit.saving.is_empty() {
            self.quit.force_close = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    /// Stops every pending close and clears the quit attempt.
    fn abort_quit(&mut self) {
        self.quit.saving.clear();
        for window in &self.documents {
            window.abort_close_after_save();
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
            self.abort_quit();
            self.quit.issue = Some(error);
            return;
        }
        if self.quit.saving.is_empty() {
            if self.documents.iter().all(|window| !window.is_dirty()) {
                self.quit.force_close = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else {
                self.abort_quit();
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
                let response = match index {
                    0 => QuitResponse::Save,
                    1 => QuitResponse::DontSave,
                    _ => QuitResponse::Cancel,
                };
                self.answer_quit_prompt(response, ui.ctx());
            }
        }

        if !self.quit.saving.is_empty() {
            let progress = self
                .documents
                .iter()
                .filter_map(|window| window.save_progress())
                .reduce(f64::max)
                .unwrap_or(0.0);
            let actions = [AlertAction::new(
                tr(self.language, "Cancel"),
                ButtonKind::Bordered,
            )];
            if modal(
                ui,
                "quit-saving",
                &tabtool_ui::document::saving_text(self.language, progress),
                "",
                &actions,
            )
            .is_some()
            {
                for window in &self.documents {
                    window.cancel_close_save();
                }
                self.abort_quit();
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
                &tabtool_ui::document::saving_text(self.language, progress),
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

//! Menu dispatch, window lifecycle and the root close policy.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use egui::{Context, ViewportCommand, ViewportId};
use tabtool_document::Document;
use tabtool_platform::OpenUrl;

use crate::documents::{DocumentWindow, WindowServices};
use crate::menu::MenuAction;
use crate::platform::interface_language;
use crate::settings_window::SettingsWindow;

use super::{DesktopApp, QuitDecision, quit_decision};

impl DesktopApp {
    pub(super) fn push_document(&mut self, document: Document, untitled_name: String) {
        let serial = self.next_window;
        self.next_window += 1;
        let viewport_id = ViewportId::from_hash_of(("tabtool-document", serial));
        let window = DocumentWindow::new(
            viewport_id,
            document,
            untitled_name,
            self.language,
            self.settings.show_format_bar,
            WindowServices {
                platform: self.platform,
                dialogs: Arc::clone(&self.dialogs),
                storage: Arc::clone(&self.storage),
                settings: self.settings.clone(),
            },
        );
        self.documents.push(window);
    }

    pub(crate) fn perform(&mut self, action: MenuAction, ctx: &Context) {
        match action {
            MenuAction::About => self.about = true,
            MenuAction::Settings => self.open_settings(),
            MenuAction::Quit => self.request_quit(ctx),
            MenuAction::New => self.new_document(),
            MenuAction::Open => self.open_from_dialog(),
            MenuAction::OpenRecent => {
                self.files.forget_missing_recent();
                self.sync_recent();
                self.recent_picker = self.files.has_recent();
            }
            MenuAction::ToggleFormatBar => {
                self.settings.show_format_bar = !self.settings.show_format_bar;
                self.settings_dirty = true;
            }
            MenuAction::Website => self.platform.open("https://tabtool.shurco.app"),
            MenuAction::License => self
                .platform
                .open("https://github.com/shurco/tabtool/blob/main/LICENSE"),
            _ => {}
        }
    }

    pub(super) fn sync_language(&mut self) {
        self.language = interface_language(&self.settings, &self.platform);
    }

    pub(super) fn handle_root_close(&mut self, ctx: &Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.quit.force_close {
            // The user already chose Don't Save or every document was saved.
            // Let the close proceed even though documents may still report
            // unsaved edits.
            return;
        }
        match quit_decision(&self.documents) {
            QuitDecision::Quit => {}
            QuitDecision::Ask => {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.quit.prompt = true;
            }
            QuitDecision::Busy => {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.quit.busy = true;
            }
        }
    }

    pub(super) fn close_finished_aux_windows(&mut self) {
        if self.about_closed.swap(false, Ordering::SeqCst) {
            self.about = false;
        }
        if self
            .settings_window_state
            .as_ref()
            .is_some_and(SettingsWindow::take_closed)
        {
            self.settings_window = false;
            self.settings_window_state = None;
            self.settings_dirty = true;
        }
    }

    pub(super) fn remove_closed(&mut self, ctx: &Context) {
        let mut closed = Vec::new();
        self.documents.retain(|window| {
            if window.should_close() {
                closed.push(window.viewport_id());
                false
            } else {
                true
            }
        });
        for viewport_id in closed {
            ctx.send_viewport_cmd_to(viewport_id, ViewportCommand::Close);
        }
    }
}

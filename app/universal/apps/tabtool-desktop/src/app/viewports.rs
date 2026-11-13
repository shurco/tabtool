//! Registering the deferred viewports and keeping the root visibility in sync.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use egui::{Context, Ui, ViewportBuilder, ViewportCommand, ViewportId};
use tabtool_i18n::tr;

use crate::app::DesktopApp;
use crate::documents::DOCUMENT_MIN_INNER_SIZE;

impl DesktopApp {
    /// Shows one deferred viewport per document and the auxiliary windows.
    pub(super) fn register_viewports(&mut self, ctx: &Context) {
        for window in &self.documents {
            let viewport_id = window.viewport_id();
            let builder = ViewportBuilder::default()
                .with_title(window.title())
                .with_min_inner_size(DOCUMENT_MIN_INNER_SIZE);
            let window = window.clone();
            ctx.show_viewport_deferred(viewport_id, builder, move |ui, _class| {
                window.render(ui);
            });
        }

        if self.about {
            let language = self.language;
            let closed = Arc::clone(&self.about_closed);
            let builder = ViewportBuilder::default()
                .with_title(tr(language, "About TabTool"))
                .with_inner_size([420.0, 320.0])
                .with_min_inner_size([320.0, 200.0]);
            ctx.show_viewport_deferred(about_viewport_id(), builder, move |ui, _class| {
                if ui.ctx().input(|input| input.viewport().close_requested()) {
                    closed.store(true, Ordering::SeqCst);
                }
                placeholder_window(ui, tr(language, "About TabTool"));
            });
        }

        if self.settings_window {
            let language = self.language;
            let closed = Arc::clone(&self.settings_closed);
            let builder = ViewportBuilder::default()
                .with_title(tr(language, "Settings"))
                .with_inner_size([520.0, 420.0])
                .with_min_inner_size([360.0, 240.0]);
            ctx.show_viewport_deferred(settings_viewport_id(), builder, move |ui, _class| {
                if ui.ctx().input(|input| input.viewport().close_requested()) {
                    closed.store(true, Ordering::SeqCst);
                }
                placeholder_window(ui, tr(language, "Settings"));
            });
        }
    }

    /// The root window is only needed for the empty state and quit alerts.
    pub(super) fn sync_root_visibility(&mut self, ctx: &Context) {
        if self.quit.force_close {
            return;
        }
        let visible = self.documents.is_empty()
            || self.quit.prompt
            || self.quit.busy
            || self.quit.issue.is_some()
            || !self.quit.saving.is_empty();
        if self.root_visible != Some(visible) {
            ctx.send_viewport_cmd(ViewportCommand::Visible(visible));
            if visible && (self.quit.prompt || self.quit.busy || self.quit.issue.is_some()) {
                ctx.send_viewport_cmd(ViewportCommand::Focus);
            }
            self.root_visible = Some(visible);
        }
    }

    /// Wakes the root logic when the next autosave or save poll is due.
    pub(super) fn schedule_repaint(&self, ctx: &Context, now: Instant) {
        let mut earliest: Option<Duration> = None;
        for window in &self.documents {
            if let Some(after) = window.repaint_after(now) {
                earliest = Some(earliest.map_or(after, |current| current.min(after)));
            }
        }
        if let Some(after) = earliest {
            ctx.request_repaint_after(after);
        }
    }
}

fn about_viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-about")
}

fn settings_viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-settings")
}

fn placeholder_window(ui: &mut Ui, title: &str) {
    let theme = tabtool_ui::theme::current(ui.ctx());
    egui::Frame::new()
        .fill(theme.colors.window_background)
        .inner_margin(theme.metrics.window_padding)
        .show(ui, |ui| {
            ui.heading(title);
        });
}

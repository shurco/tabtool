//! Registering the deferred viewports and keeping the root visibility in sync.

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Context, ViewportBuilder, ViewportCommand};

use crate::about_window;
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
            about_window::show_viewport(
                ctx,
                self.language,
                Arc::clone(&self.about_closed),
                Arc::clone(&self.about_action),
            );
        }

        if self.settings_window
            && let Some(window) = &self.settings_window_state
        {
            window.show_viewport(ctx, self.language);
        }
    }

    /// The root window is only needed for the empty state and quit alerts.
    pub(super) fn sync_root_visibility(&mut self, ctx: &Context) {
        if self.quit.force_close {
            return;
        }
        let visible = self.documents.is_empty()
            || self.recent_picker
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

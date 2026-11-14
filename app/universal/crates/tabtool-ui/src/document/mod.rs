//! The document chrome: toolbar, grid, banners, format bar, status line and
//! the read-failure overlay.
//!
//! [`DocumentView`] composes the whole document window below the menu bar. It
//! owns no document model: every mutation goes through [`Document`], and the
//! grid's display rows are translated with the shared dispatcher in
//! [`dispatch`].

mod actions;
mod banners;
mod dispatch;
mod error_overlay;
mod format_bar;
mod panels;
mod progress;
mod status_line;
mod toolbar;

use std::time::{Duration, Instant};

use egui::{Context, Frame, Ui, ViewportCommand, ViewportId};
use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::{Document, DocumentError, DocumentNotice, DocumentPhase};
use tabtool_i18n::Language;

use crate::grid::{Clipboard, GridAction, GridAppearance, GridState};
use crate::theme::current;

pub use dispatch::{apply_grid_action, sync_selection_to_document, sync_selection_to_grid};
pub use error_overlay::alert_message;
pub use format_bar::FormatBarAction;
pub use panels::grid_options;
pub use progress::{indexing_text, saving_text};
pub(crate) use status_line::rows_text;
pub use status_line::{document_summary, status_summary};

pub(crate) use actions::{
    FormatApplyOutcome, apply_format_configuration, detect_format_configuration, is_document_busy,
    poll_large_state, request_format_configuration,
};
pub use banners::{
    NeedsAttentionResponse, large_file as large_file_banner,
    needs_attention as needs_attention_banner, notice as notice_banner, stale as stale_banner,
};

/// Whether the document can be exported now.
///
/// Export needs an in-memory table: a large file stays on its private working
/// file, and Save As copies it through the platform picker instead.
pub fn can_export(document: &Document) -> bool {
    document.has_content() && !document.is_large_mode() && document.table().is_some()
}

/// The number of enabled filter criteria the toolbar badge reports.
pub fn enabled_filter_count(document: &Document) -> usize {
    document.row_filter().applied.as_ref().map_or(0, |filter| {
        filter
            .criteria
            .iter()
            .filter(|criterion| criterion.is_enabled)
            .count()
    })
}

/// A one-shot message the format bar or a large-file action raises.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromeAlert {
    /// The catalog key used as the title.
    pub title: &'static str,
    /// The localized message, including its recovery hint.
    pub message: String,
}

/// The persistent interface state of one document window.
#[derive(Default)]
pub struct DocumentUiState {
    /// A menu-initiated paste is waiting for the platform event.
    pub pending_paste: bool,
    /// When the pending paste was requested.
    pub paste_requested_at: Option<Instant>,
    /// The configuration waiting for the discard confirmation.
    pub pending_reparse: Option<CsvConfiguration>,
    /// The failure the format bar must explain.
    pub alert: Option<ChromeAlert>,
    /// A document notice the user has not dismissed.
    pub notice: Option<DocumentNotice>,
    /// When the large-file source was last checked for staleness.
    pub last_stale_check: Option<Instant>,
}

/// How long a requested paste waits for the platform event.
pub const PASTE_TIMEOUT: Duration = Duration::from_secs(2);

impl DocumentUiState {
    /// Requests one platform paste and remembers that it is outstanding.
    ///
    /// A second request while one is pending does nothing, so an empty paste
    /// event cannot loop.
    pub fn request_paste(&mut self, ctx: &Context, viewport_id: ViewportId) {
        if self.pending_paste {
            return;
        }
        self.pending_paste = true;
        self.paste_requested_at = Some(Instant::now());
        ctx.send_viewport_cmd_to(viewport_id, ViewportCommand::RequestPaste);
        ctx.request_repaint_of(viewport_id);
    }

    /// Clears a pending paste that will never receive an event.
    ///
    /// Returns `true` when the timeout fired. Without this, a platform that
    /// drops the request would keep the window armed and swallow the next
    /// unrelated paste.
    pub fn expire_pending_paste(&mut self, now: Instant) -> bool {
        let Some(requested) = self.paste_requested_at else {
            return false;
        };
        if now.saturating_duration_since(requested) < PASTE_TIMEOUT {
            return false;
        }
        self.clear_pending_paste();
        true
    }

    /// Drops the pending flag after a paste was applied or abandoned.
    pub fn clear_pending_paste(&mut self) {
        self.pending_paste = false;
        self.paste_requested_at = None;
    }

    /// Shows a failure the next frame.
    pub fn show_alert(&mut self, title: &'static str, message: String) {
        self.alert = Some(ChromeAlert { title, message });
    }
}

/// An app-level action the chrome asks its window to run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentAction {
    /// Open the export panel.
    Export,
    /// The needs-attention banner asked for the format controls.
    ShowFormatBar,
    /// A large-file control routed to the document.
    Large(LargeUiAction),
}

/// The controls that change a large document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LargeUiAction {
    /// Stop the streaming save and restore the original state.
    CancelSave,
    /// Re-read the changed file, keeping the edits.
    ReloadStale,
}

/// The document window composition.
pub struct DocumentView<'a> {
    /// The document the chrome edits.
    pub document: &'a mut Document,
    /// The persistent scroll, selection, width and editor state.
    pub state: &'a mut GridState,
    /// The chrome's own one-shot state.
    pub ui_state: &'a mut DocumentUiState,
    /// The storage a re-read or detection uses.
    pub storage: &'a dyn tabtool_document::Storage,
    /// The platform clipboard for copy and paste.
    pub clipboard: &'a mut dyn Clipboard,
    /// The interface language.
    pub language: Language,
    /// Whether the system language prefers GBK over CP1252.
    pub prefers_chinese: bool,
    /// The system locale for clipboard formatting.
    pub locale: &'a Locale,
    /// The system decimal separator for clipboard formatting.
    pub decimal_separator: &'a str,
    /// Whether the format bar is visible.
    pub show_format_bar: bool,
    /// The grid row height, alternating rows and row numbers.
    pub appearance: GridAppearance,
    /// Whether a modal owns the keyboard, so the grid must not take keys.
    pub input_blocked: bool,
    /// A failed save the window wants to explain.
    pub save_error: Option<DocumentError>,
}

impl DocumentView<'_> {
    /// Draws the document chrome; returns the actions for the window.
    pub fn show(mut self, ui: &mut Ui) -> Vec<DocumentAction> {
        let ctx = ui.ctx().clone();
        let mut actions = Vec::new();

        self.poll_large_state(&ctx);
        self.ui_state.expire_pending_paste(Instant::now());
        if let Some(notice) = self.document.take_notice() {
            self.ui_state.notice = Some(notice);
        }

        let mut paste_attempted = false;
        if self.ui_state.pending_paste {
            paste_attempted = true;
            if self.apply_pending_paste() {
                self.ui_state.clear_pending_paste();
            }
        }

        let show_content = matches!(
            self.document.phase(),
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
        ) && self.document.has_content();
        if show_content {
            let response = toolbar::show(ui, self.document, self.state, self.language);
            if !response.actions.is_empty() || response.export {
                self.commit_active_editor();
            }
            for action in response.actions {
                self.run_grid_action(action, &ctx, paste_attempted);
                paste_attempted = paste_attempted || self.ui_state.pending_paste;
            }
            if response.export {
                actions.push(DocumentAction::Export);
            }
            self.show_filter_bar(ui);
        }

        self.show_status(ui, &mut actions);
        self.show_format_bar(ui);
        self.show_alert(ui);
        self.show_notice(ui);
        self.show_needs_attention(ui, &mut actions);
        if self.show_stale(ui) {
            // The reload replaces the mapped rows; keep typed text first.
            self.commit_active_editor();
            actions.push(DocumentAction::Large(LargeUiAction::ReloadStale));
        }
        self.show_large_banner(ui);
        self.show_save_error(ui);

        let grid_actions = egui::CentralPanel::default()
            .frame(Frame::new().fill(current(ui.ctx()).colors.content_background))
            .show(ui, |ui| self.show_content(ui))
            .inner;
        for action in grid_actions {
            if paste_attempted && matches!(action, GridAction::Paste) {
                continue;
            }
            self.run_grid_action(action, &ctx, paste_attempted);
            paste_attempted = paste_attempted || self.ui_state.pending_paste;
        }

        self.show_reparse_modal(ui);
        sync_selection_to_document(self.document, self.state);
        actions
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::{DocumentUiState, PASTE_TIMEOUT};

    #[test]
    fn a_pending_paste_expires_and_frees_the_next_event() {
        let requested = Instant::now();
        let mut state = DocumentUiState {
            pending_paste: true,
            paste_requested_at: Some(requested),
            ..Default::default()
        };

        assert!(!state.expire_pending_paste(requested + PASTE_TIMEOUT / 2));
        assert!(state.pending_paste, "a fresh request must stay armed");

        assert!(state.expire_pending_paste(requested + PASTE_TIMEOUT));
        assert!(!state.pending_paste);
        assert_eq!(state.paste_requested_at, None);
    }
}

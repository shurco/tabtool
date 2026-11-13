//! Chrome actions: grid dispatch, pending paste and format changes.

use std::time::Duration;

use egui::Context;
use tabtool_core::CsvConfiguration;
use tabtool_document::{
    DocumentError, DocumentPhase, FormatChange, LargeStatus, plan_format_change,
};
use tabtool_i18n::{Language, tr};

use crate::filter::FilterAction;
use crate::grid::{ActionOutcome, GridAction};

use super::dispatch::apply_grid_action;
use super::{DocumentView, FormatBarAction, alert_message};

/// How often a large document rechecks the file at rest.
pub(crate) const STALE_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// The next repaint delay for a large document, or `None` when it is idle.
pub(crate) fn repaint_interval(is_large: bool, status: LargeStatus) -> Option<Duration> {
    match status {
        LargeStatus::Indexing(_) | LargeStatus::Saving(_) => Some(Duration::from_millis(100)),
        LargeStatus::Ready if is_large => Some(STALE_CHECK_INTERVAL),
        LargeStatus::None | LargeStatus::Ready | LargeStatus::Stale { .. } => None,
    }
}

impl DocumentView<'_> {
    pub(super) fn poll_large_state(&mut self, ctx: &Context) {
        self.document.poll_large_indexing();
        self.document.refresh_large_staleness();
        if let Some(interval) =
            repaint_interval(self.document.is_large_mode(), self.document.large_status())
        {
            ctx.request_repaint_after(interval);
        }
    }

    pub(super) fn run_grid_action(
        &mut self,
        action: GridAction,
        ctx: &Context,
        paste_handled: bool,
    ) {
        let is_paste = matches!(action, GridAction::Paste);
        if paste_handled && is_paste {
            return;
        }
        let result = apply_grid_action(
            self.document,
            self.state,
            action,
            self.clipboard,
            self.locale,
            self.decimal_separator,
        );
        match result {
            Ok(ActionOutcome::NeedsPasteText) => {
                self.ui_state.request_paste(ctx, ctx.viewport_id());
            }
            Ok(ActionOutcome::Applied) => {
                if is_paste {
                    self.ui_state.clear_pending_paste();
                }
            }
            Err(_) => {
                if is_paste {
                    self.ui_state.clear_pending_paste();
                }
            }
        }
    }

    /// Tries the outstanding paste once; `false` means the text is still
    /// missing, so the flag stays set and no second request is sent.
    pub(super) fn apply_pending_paste(&mut self) -> bool {
        match apply_grid_action(
            self.document,
            self.state,
            GridAction::Paste,
            self.clipboard,
            self.locale,
            self.decimal_separator,
        ) {
            Ok(ActionOutcome::Applied) | Err(_) => true,
            Ok(ActionOutcome::NeedsPasteText) => false,
        }
    }

    pub(super) fn apply_filter_action(&mut self, action: FilterAction) {
        if action.changes_rows() {
            self.commit_active_editor();
        }
        match action {
            FilterAction::ApplyAll => self.document.apply_all_filters(),
            FilterAction::Apply(id) => self.document.apply_filter(id),
            FilterAction::Clear => self.document.clear_filter(),
            FilterAction::Add(id) => self.document.row_filter_mut().add_criterion(id),
            FilterAction::Remove(id) => self.document.row_filter_mut().remove_criterion(id),
            FilterAction::Hide => self.document.row_filter_mut().is_bar_visible = false,
        }
    }

    /// Writes an open cell editor before a filter changes the visible rows.
    ///
    /// The commit uses the display row under the filter that is still applied,
    /// so it lands on the row the user was editing, not on whatever row takes
    /// that display position afterwards.
    fn commit_active_editor(&mut self) {
        let Some(editing) = self.state.editing.take() else {
            return;
        };
        let _ = apply_grid_action(
            self.document,
            self.state,
            editing.commit_action(),
            self.clipboard,
            self.locale,
            self.decimal_separator,
        );
    }

    pub(super) fn apply_format_action(&mut self, action: FormatBarAction) {
        match action {
            FormatBarAction::Configuration(configuration) => {
                self.request_configuration(configuration);
            }
            FormatBarAction::Detect => self.detect_format(),
        }
    }

    pub(super) fn request_configuration(&mut self, configuration: CsvConfiguration) {
        let from = self.document.configuration().clone();
        match plan_format_change(&from, &configuration) {
            FormatChange::None => {}
            FormatChange::DecimalMark(mark) if self.document.has_content() => {
                self.document.set_decimal_mark(&mark);
            }
            FormatChange::Header(enabled) if self.document.has_content() => {
                self.document.set_header_enabled(enabled);
            }
            _ => {
                let needs_confirmation = self.document.file_path().is_some()
                    && self.document.has_content()
                    && self.document.has_unsaved_changes();
                if needs_confirmation {
                    self.ui_state.pending_reparse = Some(configuration);
                } else {
                    self.apply_configuration(&configuration, true);
                }
            }
        }
    }

    pub(super) fn apply_configuration(&mut self, configuration: &CsvConfiguration, force: bool) {
        match self
            .document
            .apply_parsing_configuration(configuration.clone(), force, self.storage)
        {
            Ok(()) => self.ui_state.alert = None,
            Err(DocumentError::RequiresConfirmation) => self.ui_state.show_alert(
                "Re-read Interrupted",
                tr(
                    self.language,
                    "The document changed while it was being re-read, so the new format was not applied. Try again.",
                )
                .to_owned(),
            ),
            Err(error) => self.ui_state.show_alert(
                "Format Not Applied",
                alert_message(self.language, error),
            ),
        }
    }

    pub(super) fn detect_format(&mut self) {
        if self.document.file_path().is_none() {
            return;
        }
        match self
            .document
            .detect_format_from_disk(self.storage, matches!(self.language, Language::ZhHans))
        {
            Ok(detected) => {
                if !self.document.has_content() {
                    self.apply_configuration(&detected, true);
                } else if detected != *self.document.configuration() {
                    self.request_configuration(detected);
                }
            }
            Err(error) => self
                .ui_state
                .show_alert("Format Not Detected", alert_message(self.language, error)),
        }
    }

    pub(super) fn is_busy(&self) -> bool {
        matches!(self.document.phase(), DocumentPhase::Loading)
            || matches!(
                self.document.large_status(),
                LargeStatus::Indexing(_) | LargeStatus::Saving(_)
            )
            || self.ui_state.pending_reparse.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{STALE_CHECK_INTERVAL, repaint_interval};
    use std::time::Duration;
    use tabtool_document::LargeStatus;

    #[test]
    fn only_large_documents_poll_at_rest() {
        assert_eq!(repaint_interval(false, LargeStatus::Ready), None);
        assert_eq!(
            repaint_interval(true, LargeStatus::Ready),
            Some(STALE_CHECK_INTERVAL)
        );
        assert_eq!(
            repaint_interval(true, LargeStatus::Stale { appended: true }),
            None
        );
        assert_eq!(
            repaint_interval(true, LargeStatus::Indexing(0.5)),
            Some(Duration::from_millis(100))
        );
        assert_eq!(
            repaint_interval(true, LargeStatus::Saving(0.5)),
            Some(Duration::from_millis(100))
        );
    }
}

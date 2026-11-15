//! Chrome actions: grid dispatch, pending paste and format changes.

mod format;

use std::time::{Duration, Instant};

use egui::Context;
use tabtool_document::{Document, DocumentPhase, LargeStatus};

use crate::filter::FilterAction;
use crate::grid::{ActionOutcome, GridAction};

use super::DocumentView;
use super::dispatch::apply_grid_action;

pub(crate) use format::{
    FormatApplyOutcome, apply_format_configuration, detect_format_configuration,
    request_format_configuration,
};

/// How often a large document rechecks the file at rest.
pub(crate) const STALE_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Whether a grid command changes rows, columns or their order.
///
/// The context menu returns these next to the editor's own commit action, so
/// the editor text must be written first; otherwise an inserted row takes the
/// text meant for the row it came from.
fn needs_editor_commit(action: &GridAction) -> bool {
    matches!(
        action,
        GridAction::AddRowAbove
            | GridAction::AddRowBelow
            | GridAction::AddColumnLeft
            | GridAction::AddColumnRight
            | GridAction::DeleteSelection
            | GridAction::MoveRows { .. }
            | GridAction::MoveColumn { .. }
            | GridAction::Sort { .. }
            | GridAction::RemoveSort
            | GridAction::RenameColumn { .. }
            | GridAction::Paste
    )
}

/// The next repaint delay for a large document, or `None` when it is idle.
pub(crate) fn repaint_interval(is_large: bool, status: LargeStatus) -> Option<Duration> {
    match status {
        LargeStatus::Indexing(_) | LargeStatus::Saving(_) => Some(Duration::from_millis(100)),
        LargeStatus::Ready if is_large => Some(STALE_CHECK_INTERVAL),
        LargeStatus::None | LargeStatus::Ready | LargeStatus::Stale { .. } => None,
    }
}

/// Whether the document is loading, indexing or saving.
pub(crate) fn is_document_busy(document: &Document) -> bool {
    matches!(document.phase(), DocumentPhase::Loading)
        || matches!(
            document.large_status(),
            LargeStatus::Indexing(_) | LargeStatus::Saving(_)
        )
}

/// Polls a large document's index and at-rest staleness.
///
/// The desktop window and the touch editor share the rule: poll while busy,
/// recheck the file at most once a second at rest, and request repaints while a
/// job runs.
pub(crate) fn poll_large_state(
    document: &mut Document,
    last_stale_check: &mut Option<Instant>,
    ctx: &Context,
) {
    document.poll_large_indexing();
    let large_ready =
        document.is_large_mode() && matches!(document.large_status(), LargeStatus::Ready);
    let now = Instant::now();
    if !large_ready
        || last_stale_check
            .is_none_or(|checked| now.saturating_duration_since(checked) >= STALE_CHECK_INTERVAL)
    {
        document.refresh_large_staleness();
        if large_ready {
            *last_stale_check = Some(now);
        }
    }
    if let Some(interval) = repaint_interval(document.is_large_mode(), document.large_status()) {
        ctx.request_repaint_after(interval);
    }
}

impl DocumentView<'_> {
    pub(super) fn poll_large_state(&mut self, ctx: &Context) {
        poll_large_state(self.document, &mut self.ui_state.last_stale_check, ctx);
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
        if needs_editor_commit(&action) {
            self.commit_active_editor();
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
        if action.changes_rows() {
            // Display indexes from the previous filter no longer name the same
            // rows. Rebuild them from the document's model selection before
            // the grid paints or the end-of-frame sync writes them back.
            super::dispatch::sync_selection_to_grid(self.document, self.state);
        }
    }

    /// Writes an open cell editor before a filter or chrome action changes the
    /// document under it.
    ///
    /// The commit uses the display row under the filter that is still applied,
    /// so it lands on the row the user was editing, not on whatever row takes
    /// that display position afterwards.
    pub(super) fn commit_active_editor(&mut self) {
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

    pub(super) fn is_busy(&self) -> bool {
        is_document_busy(self.document) || self.ui_state.pending_reparse.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{STALE_CHECK_INTERVAL, needs_editor_commit, repaint_interval};
    use crate::grid::GridAction;
    use std::time::Duration;
    use tabtool_document::LargeStatus;

    #[test]
    fn structure_commands_commit_the_editor_first() {
        for action in [
            GridAction::AddRowAbove,
            GridAction::AddRowBelow,
            GridAction::AddColumnLeft,
            GridAction::AddColumnRight,
            GridAction::DeleteSelection,
            GridAction::RemoveSort,
            GridAction::Paste,
        ] {
            assert!(needs_editor_commit(&action), "{action:?} must commit first");
        }
        for action in [GridAction::Copy, GridAction::SelectAll] {
            assert!(
                !needs_editor_commit(&action),
                "{action:?} must not commit first"
            );
        }
    }

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

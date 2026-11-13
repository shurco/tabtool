//! Everything the menu enables or disables, read once per frame.

use tabtool_core::shifted;
use tabtool_document::{Document, DocumentPhase, LargeStatus, RevertDecision, plan_revert};

/// The document capabilities the menu reads.
///
/// Each field mirrors one `AppCommands.swift` enablement expression. A window
/// without a document uses [`Capabilities::none`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Capabilities {
    /// A document is focused.
    pub document: bool,
    /// Save writes in place.
    pub save: bool,
    /// Save As asks for a destination.
    pub save_as: bool,
    /// Revert re-reads the saved file.
    pub revert: bool,
    /// Export writes a copy.
    pub export: bool,
    /// Undo has an entry.
    pub undo: bool,
    /// Redo has an entry.
    pub redo: bool,
    /// Cut copies and deletes the selection.
    pub cut: bool,
    /// Copy writes the selection as TSV.
    pub copy: bool,
    /// Paste inserts clipboard TSV.
    pub paste: bool,
    /// Delete removes the selection.
    pub delete: bool,
    /// Select All selects every row.
    pub select_all: bool,
    /// Rows can be inserted.
    pub insert_rows: bool,
    /// Selected rows can be deleted.
    pub delete_rows: bool,
    /// Columns can be inserted.
    pub add_column: bool,
    /// Selected columns can be deleted.
    pub delete_columns: bool,
    /// The selected column can be renamed.
    pub rename_column: bool,
    /// The selected column can be sorted.
    pub sort: bool,
    /// The active sort can be removed.
    pub remove_sort: bool,
    /// The rows can move up.
    pub move_row_up: bool,
    /// The rows can move down.
    pub move_row_down: bool,
    /// The columns can move left.
    pub move_column_left: bool,
    /// The columns can move right.
    pub move_column_right: bool,
    /// The filter bar and its commands are available.
    pub filter: bool,
    /// The filter bar is visible.
    pub filter_visible: bool,
}

impl Capabilities {
    /// No document is focused; every document command is disabled.
    pub fn none() -> Self {
        Self::default()
    }

    /// Reads the capabilities of `document`.
    pub fn of(document: &Document) -> Self {
        let table = document.table();
        let editable = table.is_some()
            && matches!(
                document.phase(),
                DocumentPhase::Ready | DocumentPhase::NeedsAttention(_)
            );
        let large_ready = document.is_large_mode() && document.large_status() == LargeStatus::Ready;
        let selection = document.selection();
        let column_count = table.map_or(0, |table| table.columns.len());
        let row_count = table.map_or(0, |table| table.row_count());
        let selected_column = (selection.columns.len() == 1)
            .then(|| selection.columns.first().copied())
            .flatten()
            .filter(|&column| column < column_count);
        let has_selection = !selection.rows.is_empty() || !selection.columns.is_empty();
        let selected_columns: std::collections::BTreeSet<usize> = selection
            .columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();

        Self {
            document: true,
            save: document.can_save() || large_ready,
            save_as: (editable || large_ready)
                && !matches!(
                    document.large_status(),
                    LargeStatus::Saving(_) | LargeStatus::Indexing(_)
                ),
            revert: plan_revert(document) != RevertDecision::Unavailable,
            export: editable,
            undo: document.can_undo(),
            redo: document.can_redo(),
            cut: editable && has_selection,
            copy: document.has_content() && has_selection,
            paste: editable,
            delete: document.can_delete_selection(),
            select_all: editable && row_count > 0,
            insert_rows: document.can_insert_rows(),
            delete_rows: document.can_delete_selected_rows(),
            add_column: editable,
            delete_columns: editable && column_count > 1 && !selected_columns.is_empty(),
            rename_column: selected_column.is_some_and(|column| document.can_rename_column(column)),
            sort: editable && selected_column.is_some(),
            remove_sort: editable && document.active_sort().is_some(),
            move_row_up: document.can_move_selected_rows(-1),
            move_row_down: document.can_move_selected_rows(1),
            move_column_left: editable && shifted(column_count, &selected_columns, -1).is_some(),
            move_column_right: editable && shifted(column_count, &selected_columns, 1).is_some(),
            filter: document.can_filter(),
            filter_visible: document.row_filter().is_bar_visible,
        }
    }
}

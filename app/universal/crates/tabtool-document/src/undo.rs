//! Undo and redo history for document mutations.
//!
//! Structural actions store one [`StateSnapshot`] of the state before the
//! action; cell edits store only the old and new text. Undo and redo restore a
//! snapshot or rewrite one cell, then bump the revision and refresh the write
//! gate. A new action clears the redo stack.

use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable};

use crate::Document;
use crate::types::{ActiveSort, GridSelection};

/// The user action an undo entry belongs to.
///
/// The key is stable English text, not localized: the UI maps it to a display
/// name and falls back to a generic Undo/Redo title when it does not know the
/// key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UndoAction {
    /// One cell's text changed.
    EditCell,
    /// A blank row was inserted.
    AddRow,
    /// A blank column was inserted.
    AddColumn,
    /// Rows were deleted.
    DeleteRows,
    /// Columns were deleted.
    DeleteColumns,
    /// Rows were reordered.
    MoveRows,
    /// One or more columns were reordered.
    MoveColumn,
    /// Clipboard text was pasted.
    PasteData,
    /// The decimal mark changed.
    ChangeFormat,
    /// Header mode was toggled.
    ToggleHeader,
    /// A column was renamed.
    RenameColumn,
    /// Rows were sorted.
    SortRows,
    /// The row sort was removed and the source order restored.
    RemoveSort,
    /// The document was reverted to its saved content.
    RevertToSaved,
}

impl UndoAction {
    /// The stable, non-localized key the UI maps to a display name.
    pub fn key(self) -> &'static str {
        match self {
            Self::EditCell => "editCell",
            Self::AddRow => "addRow",
            Self::AddColumn => "addColumn",
            Self::DeleteRows => "deleteRows",
            Self::DeleteColumns => "deleteColumns",
            Self::MoveRows => "moveRows",
            Self::MoveColumn => "moveColumn",
            Self::PasteData => "pasteData",
            Self::ChangeFormat => "changeFormat",
            Self::ToggleHeader => "toggleHeader",
            Self::RenameColumn => "renameColumn",
            Self::SortRows => "sortRows",
            Self::RemoveSort => "removeSort",
            Self::RevertToSaved => "revertToSaved",
        }
    }
}

/// The document state captured before a structural or format action.
///
/// The table is shared behind an [`Arc`], so an entry costs no cell copies and
/// a column identity change remains visible through
/// [`CsvTable::has_same_column_order`].
pub(crate) struct StateSnapshot {
    pub(crate) table: Arc<CsvTable>,
    pub(crate) configuration: CsvConfiguration,
    pub(crate) active_sort: Option<ActiveSort>,
    pub(crate) selection: GridSelection,
    /// Whether saved output started with a UTF-8 byte order mark.
    pub(crate) includes_utf8_bom: bool,
}

/// One entry of the undo or redo stack.
pub(crate) enum UndoEntry {
    /// A targeted cell change: undo writes `old`, redo writes `new`.
    Cell {
        action: UndoAction,
        row: usize,
        column: usize,
        old: String,
        new: String,
    },
    /// The whole document state before the action.
    State {
        action: UndoAction,
        snapshot: StateSnapshot,
    },
    /// A targeted large-file cell change, keyed by physical (file) row.
    LargeCell {
        row: usize,
        column: usize,
        old: String,
        new: String,
    },
    /// A large-file header toggle. `enabled` is the value this entry applies.
    LargeHeader { enabled: bool },
}

impl UndoEntry {
    /// The user action this entry belongs to.
    fn action(&self) -> UndoAction {
        match self {
            Self::Cell { action, .. } | Self::State { action, .. } => *action,
            Self::LargeCell { .. } => UndoAction::EditCell,
            Self::LargeHeader { .. } => UndoAction::ToggleHeader,
        }
    }

    /// Whether this entry belongs to the content mode the document has now.
    fn applies_to(&self, document: &Document) -> bool {
        match self {
            Self::Cell { .. } | Self::State { .. } => document.table.is_some(),
            Self::LargeCell { .. } | Self::LargeHeader { .. } => document.large.is_some(),
        }
    }
}

impl Document {
    /// Whether there is an action to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_entries.is_empty()
    }

    /// Whether there is an action to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_entries.is_empty()
    }

    /// The action the next undo would reverse, if any.
    pub fn undo_action(&self) -> Option<UndoAction> {
        self.undo_entries.last().map(UndoEntry::action)
    }

    /// Undoes the most recent action.
    ///
    /// Returns `false` when the history is empty, no content is loaded, or the
    /// entry belongs to a content mode the document no longer has. A refused
    /// large-file edit raises [`crate::DocumentNotice::EditRejected`] and keeps
    /// the entry on the undo stack. A successful undo records the resulting
    /// state for redo.
    pub fn undo(&mut self) -> bool {
        if self.table.is_none() && self.large.is_none() {
            return false;
        }
        let Some(entry) = self.undo_entries.pop() else {
            return false;
        };
        if !entry.applies_to(self) {
            self.undo_entries.push(entry);
            return false;
        }

        match entry {
            UndoEntry::Cell {
                action,
                row,
                column,
                old,
                new,
            } => {
                self.replace_cell(row, column, &old);
                self.refresh_filter();
                self.redo_entries.push(UndoEntry::Cell {
                    action,
                    row,
                    column,
                    old,
                    new,
                });
            }
            UndoEntry::State { action, snapshot } => {
                let Some(after) = self.capture_state() else {
                    return false;
                };
                self.restore_state(snapshot);
                self.redo_entries.push(UndoEntry::State {
                    action,
                    snapshot: after,
                });
            }
            UndoEntry::LargeCell {
                row,
                column,
                old,
                new,
            } => {
                if let Err(error) = self.apply_large_cell_at(row, column, &old) {
                    self.set_notice(crate::DocumentNotice::EditRejected(error));
                    self.undo_entries.push(UndoEntry::LargeCell {
                        row,
                        column,
                        old,
                        new,
                    });
                    return false;
                }
                self.redo_entries.push(UndoEntry::LargeCell {
                    row,
                    column,
                    old,
                    new,
                });
            }
            UndoEntry::LargeHeader { enabled } => {
                if !self.apply_large_header_value(enabled) {
                    self.undo_entries.push(UndoEntry::LargeHeader { enabled });
                    return false;
                }
                self.redo_entries
                    .push(UndoEntry::LargeHeader { enabled: !enabled });
            }
        }
        true
    }

    /// Redoes the most recently undone action.
    ///
    /// Returns `false` when there is nothing to redo, no content is loaded, or
    /// the entry belongs to a content mode the document no longer has. A
    /// refused large-file edit raises [`crate::DocumentNotice::EditRejected`]
    /// and keeps the entry on the redo stack. A successful redo records the
    /// resulting state for another undo.
    pub fn redo(&mut self) -> bool {
        if self.table.is_none() && self.large.is_none() {
            return false;
        }
        let Some(entry) = self.redo_entries.pop() else {
            return false;
        };
        if !entry.applies_to(self) {
            self.redo_entries.push(entry);
            return false;
        }

        match entry {
            UndoEntry::Cell {
                action,
                row,
                column,
                old,
                new,
            } => {
                self.replace_cell(row, column, &new);
                self.refresh_filter();
                self.undo_entries.push(UndoEntry::Cell {
                    action,
                    row,
                    column,
                    old,
                    new,
                });
            }
            UndoEntry::State { action, snapshot } => {
                let Some(before) = self.capture_state() else {
                    return false;
                };
                self.restore_state(snapshot);
                self.undo_entries.push(UndoEntry::State {
                    action,
                    snapshot: before,
                });
            }
            UndoEntry::LargeCell {
                row,
                column,
                old,
                new,
            } => {
                if let Err(error) = self.apply_large_cell_at(row, column, &new) {
                    self.set_notice(crate::DocumentNotice::EditRejected(error));
                    self.redo_entries.push(UndoEntry::LargeCell {
                        row,
                        column,
                        old,
                        new,
                    });
                    return false;
                }
                self.undo_entries.push(UndoEntry::LargeCell {
                    row,
                    column,
                    old,
                    new,
                });
            }
            UndoEntry::LargeHeader { enabled } => {
                if !self.apply_large_header_value(enabled) {
                    self.redo_entries.push(UndoEntry::LargeHeader { enabled });
                    return false;
                }
                self.undo_entries
                    .push(UndoEntry::LargeHeader { enabled: !enabled });
            }
        }
        true
    }

    /// Captures the current table and view-independent state for undo.
    pub(crate) fn capture_state(&self) -> Option<StateSnapshot> {
        let table = self.table.as_ref().map(Arc::clone)?;
        Some(StateSnapshot {
            table,
            configuration: self.configuration.clone(),
            active_sort: self.active_sort.clone(),
            selection: self.selection.clone(),
            includes_utf8_bom: self.had_utf8_bom,
        })
    }

    /// Stores the state before a structural or format action.
    pub(crate) fn record_state_change(&mut self, action: UndoAction, snapshot: StateSnapshot) {
        self.undo_entries
            .push(UndoEntry::State { action, snapshot });
        self.redo_entries.clear();
    }

    /// Stores a targeted cell change.
    pub(crate) fn record_cell_change(
        &mut self,
        action: UndoAction,
        row: usize,
        column: usize,
        old: String,
        new: String,
    ) {
        self.undo_entries.push(UndoEntry::Cell {
            action,
            row,
            column,
            old,
            new,
        });
        self.redo_entries.clear();
    }

    /// Stores a targeted large-file cell change, keyed by physical row.
    pub(crate) fn record_large_cell_change(
        &mut self,
        row: usize,
        column: usize,
        old: String,
        new: String,
    ) {
        self.undo_entries.push(UndoEntry::LargeCell {
            row,
            column,
            old,
            new,
        });
        self.redo_entries.clear();
    }

    /// Stores the header value a large-file toggle must restore.
    pub(crate) fn record_large_header_change(&mut self, previous: bool) {
        self.undo_entries
            .push(UndoEntry::LargeHeader { enabled: previous });
        self.redo_entries.clear();
    }

    /// Publishes a captured state as a new document revision.
    fn restore_state(&mut self, snapshot: StateSnapshot) {
        self.table = Some(snapshot.table);
        self.configuration = snapshot.configuration;
        self.active_sort = snapshot.active_sort;
        self.selection = snapshot.selection;
        self.had_utf8_bom = snapshot.includes_utf8_bom;
        self.bump_revision();
        self.refresh_phase();
        self.refresh_filter();
    }
}

#[cfg(test)]
mod tests {
    use tabtool_core::{CsvConfiguration, SortDirection};

    use super::*;

    fn document() -> Document {
        let configuration = CsvConfiguration::default();
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(b"b\na\nc\n", None, Some(configuration), false)
            .expect("load the fixture");
        document
    }

    fn document_with_bom() -> Document {
        let configuration = CsvConfiguration::default();
        let mut input = vec![0xEF, 0xBB, 0xBF];
        input.extend_from_slice(b"b\na\nc\n");
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(&input, None, Some(configuration), false)
            .expect("load the fixture");
        document
    }

    fn sort() -> ActiveSort {
        ActiveSort {
            column_id: 1,
            direction: SortDirection::Ascending,
            source_order: vec![1, 0, 2],
        }
    }

    #[test]
    fn undoing_a_structural_change_restores_the_sort_and_configuration() {
        let mut document = document();
        document.active_sort = Some(sort());
        document.configuration.decimal_mark = ",".to_string();

        document.add_row_below_selection();
        assert_eq!(document.active_sort, None);

        assert!(document.undo());
        assert_eq!(document.active_sort, Some(sort()));
        assert_eq!(document.configuration.decimal_mark, ",");
    }

    #[test]
    fn undoing_a_structural_change_restores_the_utf8_bom_flag() {
        let mut document = document_with_bom();
        assert!(document.snapshot().expect("snapshot").includes_utf8_bom);

        document.add_row_below_selection();
        // A later format change or re-read may clear the flag without changing
        // the table; undo must restore the state captured before the action.
        document.had_utf8_bom = false;

        assert!(document.undo());
        assert!(document.snapshot().expect("snapshot").includes_utf8_bom);
    }

    #[test]
    fn undoing_a_cell_edit_keeps_the_active_sort() {
        let mut document = document();
        document.active_sort = Some(sort());

        document.set_cell(0, 0, "z");
        assert!(document.undo());

        assert_eq!(document.table().expect("table").cell(0, 0), "b");
        assert_eq!(document.active_sort, Some(sort()));
    }
}

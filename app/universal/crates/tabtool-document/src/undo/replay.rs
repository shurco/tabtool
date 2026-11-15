//! Applying undo and redo entries to the document.

use super::{StateSnapshot, UndoEntry};
use crate::Document;

impl Document {
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

//! The publish path every structural or format action shares.

use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable};

use crate::Document;
use crate::types::{ActiveSort, GridSelection};
use crate::undo::UndoAction;

impl Document {
    /// Applies a table mutation and records the previous state for undo.
    ///
    /// Returns `false` without a revision, sort change or undo entry when the
    /// mutation leaves the content and column order unchanged.
    pub(crate) fn replace_table(
        &mut self,
        action: UndoAction,
        mutate: impl FnOnce(&mut CsvTable),
        next_sort: Option<ActiveSort>,
    ) -> bool {
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return false;
        };
        let mut updated = Arc::clone(&current);
        mutate(Arc::make_mut(&mut updated));
        let configuration = self.configuration.clone();
        let includes_utf8_bom = self.had_utf8_bom;
        self.publish_content(action, updated, configuration, includes_utf8_bom, next_sort)
    }

    /// Applies a table prepared by the caller and records the previous state.
    ///
    /// Sorting and pasting build their table from a COW clone, so they hand
    /// the prepared value in instead of mutating a second copy.
    pub(crate) fn replace_table_with(
        &mut self,
        action: UndoAction,
        updated: Arc<CsvTable>,
        next_sort: Option<ActiveSort>,
    ) -> bool {
        if self.table.is_none() {
            return false;
        }
        let configuration = self.configuration.clone();
        let includes_utf8_bom = self.had_utf8_bom;
        self.publish_content(action, updated, configuration, includes_utf8_bom, next_sort)
    }

    /// Publishes prepared table, format and BOM state after the no-op check.
    ///
    /// This is the one path every structural or format action shares, so the
    /// revision, phase, filter refresh and undo snapshot cannot go stale.
    /// Equal content, column order, configuration and BOM record nothing,
    /// matching the Swift `applyChange` early return. A document without a
    /// table still accepts a first table, without an undo snapshot.
    pub(crate) fn publish_content(
        &mut self,
        action: UndoAction,
        updated: Arc<CsvTable>,
        configuration: CsvConfiguration,
        includes_utf8_bom: bool,
        next_sort: Option<ActiveSort>,
    ) -> bool {
        // A large document has exactly one content mode: a table can never
        // replace the mapped source.
        if self.large.is_some() {
            return false;
        }
        if let Some(current) = self.table.as_ref()
            && current.as_ref() == updated.as_ref()
            && current.has_same_column_order(updated.as_ref())
            && self.configuration == configuration
            && self.had_utf8_bom == includes_utf8_bom
        {
            return false;
        }

        let before = self.capture_state();
        self.table = Some(updated);
        self.configuration = configuration;
        self.had_utf8_bom = includes_utf8_bom;
        self.active_sort = next_sort;
        self.trim_selection();
        self.bump_revision();
        self.refresh_phase();
        self.refresh_filter();
        if let Some(before) = before {
            self.record_state_change(action, before);
        }
        true
    }

    /// Drops selected indexes that no longer exist after a change.
    ///
    /// A large document trims against the rows and columns its source shows
    /// now; a document with neither content mode clears the selection.
    pub(crate) fn trim_selection(&mut self) {
        if let Some(large) = &self.large {
            let row_count = large.source.row_count();
            let column_count = large.source.column_count();
            self.selection.rows.retain(|&row| row < row_count);
            self.selection
                .columns
                .retain(|&column| column < column_count);
            return;
        }
        let Some(table) = self.table.as_ref() else {
            self.selection = GridSelection::default();
            return;
        };
        let row_count = table.row_count();
        let column_count = table.columns.len();
        self.selection.rows.retain(|&row| row < row_count);
        self.selection
            .columns
            .retain(|&column| column < column_count);
    }
}

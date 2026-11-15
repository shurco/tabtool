//! The document side of the row filter: availability, evaluation against
//! the table, the row-structure lock, the export table and the selection
//! trim.

use std::collections::{BTreeSet, HashSet};

use tabtool_core::{CriterionId, CsvTable};

use super::RowFilterModel;
use crate::Document;

impl Document {
    /// The row filter model of this document.
    pub fn row_filter(&self) -> &RowFilterModel {
        &self.row_filter
    }

    /// The row filter model of this document, for mutation by the filter bar.
    pub fn row_filter_mut(&mut self) -> &mut RowFilterModel {
        &mut self.row_filter
    }

    /// Whether the filter bar and its commands are available.
    ///
    /// Large documents and documents that are still loading or failed to read
    /// cannot filter.
    pub fn can_filter(&self) -> bool {
        !self.is_large_mode() && self.can_edit_content()
    }

    /// Applies every enabled draft criterion to the current table.
    pub fn apply_all_filters(&mut self) {
        if !self.can_filter() {
            return;
        }
        let Some(table) = self.table.as_deref() else {
            return;
        };
        self.row_filter
            .apply_all(table, &self.configuration.decimal_mark);
        self.keep_selection_visible();
    }

    /// Applies one draft criterion alone to the current table.
    pub fn apply_filter(&mut self, criterion_id: CriterionId) {
        if !self.can_filter() {
            return;
        }
        let Some(table) = self.table.as_deref() else {
            return;
        };
        self.row_filter
            .apply(criterion_id, table, &self.configuration.decimal_mark);
        self.keep_selection_visible();
    }

    /// Removes the applied filter and resets the draft criteria.
    pub fn clear_filter(&mut self) {
        self.row_filter.clear();
    }

    /// Re-evaluates the applied filter after the table was replaced or undone.
    ///
    /// Cell edits skip this on purpose, so an edited row stays in view.
    pub fn refresh_filter(&mut self) {
        let Some(table) = self.table.as_deref() else {
            return;
        };
        self.row_filter
            .refresh(table, &self.configuration.decimal_mark);
        self.keep_selection_visible();
    }

    /// Shows or hides the filter bar; an applied filter stays active.
    pub fn toggle_filter_bar(&mut self) {
        if !self.can_filter() {
            return;
        }
        self.row_filter.is_bar_visible = !self.row_filter.is_bar_visible;
    }

    /// The table as shown, for export: only the rows that pass the applied
    /// filter, or the whole table when no filter is applied.
    pub fn visible_table(&self) -> Option<CsvTable> {
        let table = self.table.as_deref()?;
        let Some(visible_rows) = self.row_filter.visible_rows.as_ref() else {
            return Some(table.clone());
        };
        let visible: HashSet<usize> = visible_rows.iter().copied().collect();
        let hidden: BTreeSet<usize> = (0..table.row_count())
            .filter(|row| !visible.contains(row))
            .collect();
        let mut visible_table = table.clone();
        visible_table.delete_rows(&hidden);
        Some(visible_table)
    }

    /// Whether rows can be added, deleted or moved.
    ///
    /// A filtered view shows a subset of the rows, so those edits wait until
    /// the filter is cleared. Columns stay editable.
    pub fn can_edit_row_structure(&self) -> bool {
        !self.row_filter.is_filtering()
    }

    /// Whether a row can be inserted through the toolbar and menus.
    pub fn can_insert_rows(&self) -> bool {
        self.can_edit_content() && self.can_edit_row_structure()
    }

    /// Whether a valid row selection can be deleted.
    pub fn can_delete_selected_rows(&self) -> bool {
        let Some(table) = self.table.as_deref() else {
            return false;
        };
        self.can_edit_content()
            && self.can_edit_row_structure()
            && self
                .selection
                .rows
                .iter()
                .any(|&row| row < table.row_count())
    }

    /// Whether the current selection can be deleted.
    ///
    /// Rows cannot be deleted while a filter is applied. A column-only
    /// selection is disabled when the table has a single column, because the
    /// document keeps that column.
    pub fn can_delete_selection(&self) -> bool {
        let Some(table) = self.table.as_deref() else {
            return false;
        };
        if !self.can_edit_content() {
            return false;
        }
        if self
            .selection
            .rows
            .iter()
            .any(|&row| row < table.row_count())
        {
            return self.can_edit_row_structure();
        }
        self.selection
            .columns
            .iter()
            .any(|&column| column < table.columns.len())
            && table.columns.len() > 1
    }

    /// Whether the selected rows can move one step by `by`.
    pub fn can_move_selected_rows(&self, by: isize) -> bool {
        let Some(table) = self.table.as_deref() else {
            return false;
        };
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return false;
        }
        let row_count = table.row_count();
        let selected: BTreeSet<usize> = self
            .selection
            .rows
            .iter()
            .copied()
            .filter(|&row| row < row_count)
            .collect();
        tabtool_core::shifted(row_count, &selected, by).is_some()
    }

    /// Drops selected rows that the filter hides.
    fn keep_selection_visible(&mut self) {
        let Some(visible_rows) = self.row_filter.visible_rows.as_ref() else {
            return;
        };
        if self.selection.rows.is_empty() {
            return;
        }
        let visible: HashSet<usize> = visible_rows.iter().copied().collect();
        self.selection.rows.retain(|row| visible.contains(row));
    }
}

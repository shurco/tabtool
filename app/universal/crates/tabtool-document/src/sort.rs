//! Row sorting commands and their removable sort state.
//!
//! A sort records [`ActiveSort`] so removing it applies the inverse row
//! permutation. A sort that leaves the rows unchanged still sets the arrow,
//! matching the Swift document, but records no undo step.

use std::collections::BTreeSet;
use std::sync::Arc;

use tabtool_core::{Locale, SortDirection, row_order_sorted_by};

use crate::Document;
use crate::types::{ActiveSort, GridSelection};
use crate::undo::UndoAction;

impl Document {
    /// Sorts the rows by `column`, keeping the sort removable.
    ///
    /// A sort that leaves the rows unchanged records no undo step; the sort
    /// arrow is still set. The sorted column becomes the selection.
    pub fn sort_rows(&mut self, column: usize, direction: SortDirection, locale: &Locale) {
        if !self.can_edit_content() {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let Some(sorted_column) = current.columns.get(column) else {
            return;
        };

        let order = row_order_sorted_by(
            &current,
            column,
            direction,
            &self.configuration.decimal_mark,
            locale,
        );
        let previous_source = self.previous_source_order(current.row_count());
        let source_order = order
            .iter()
            .map(|&row| previous_source.get(row).copied().unwrap_or(row))
            .collect();
        let next_sort = ActiveSort {
            column_id: sorted_column.id,
            direction,
            source_order,
        };

        let mut reordered = Arc::clone(&current);
        Arc::make_mut(&mut reordered).reorder_rows(&order);
        self.replace_table_with(UndoAction::SortRows, reordered, Some(next_sort.clone()));
        self.active_sort = Some(next_sort);
        self.selection = GridSelection {
            rows: BTreeSet::new(),
            columns: BTreeSet::from([column]),
        };
    }

    /// Restores the pre-sort row order and clears the sort.
    pub fn remove_sort(&mut self) {
        if !self.can_edit_content() {
            return;
        }
        let Some(sort) = self.active_sort.clone() else {
            return;
        };
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };

        let mut inverse = vec![0; sort.source_order.len()];
        for (current_index, &source) in sort.source_order.iter().enumerate() {
            if let Some(slot) = inverse.get_mut(source) {
                *slot = current_index;
            }
        }

        let mut restored = Arc::clone(&current);
        Arc::make_mut(&mut restored).reorder_rows(&inverse);
        self.replace_table_with(UndoAction::SortRows, restored, None);
        self.active_sort = None;
    }

    /// Advances `column` through none, ascending, descending, none.
    pub fn cycle_sort(&mut self, column: usize, locale: &Locale) {
        match self.sort_direction(column) {
            None => self.sort_rows(column, SortDirection::Ascending, locale),
            Some(SortDirection::Ascending) => {
                self.sort_rows(column, SortDirection::Descending, locale);
            }
            Some(SortDirection::Descending) => self.remove_sort(),
        }
    }

    /// The sort direction of `column` when it is the sorted column.
    pub fn sort_direction(&self, column: usize) -> Option<SortDirection> {
        let table = self.table.as_ref()?;
        let column_id = table.columns.get(column)?.id;
        let sort = self.active_sort.as_ref()?;
        (sort.column_id == column_id).then_some(sort.direction)
    }

    /// The pre-sort index of every current row, or the identity order.
    ///
    /// The stored order is trusted only when it still matches the row count;
    /// structural changes clear the sort, so a mismatch means stale state.
    fn previous_source_order(&self, row_count: usize) -> Vec<usize> {
        match self.active_sort.as_ref() {
            Some(sort) if sort.source_order.len() == row_count => sort.source_order.clone(),
            _ => (0..row_count).collect(),
        }
    }
}

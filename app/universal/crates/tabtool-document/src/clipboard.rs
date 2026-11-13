//! Clipboard copy, the copy cell limit and TSV paste.
//!
//! Copy reads the document's [`RowSource`] in visual column order and
//! localizes numeric cells. Paste parses TSV without quote handling, converts
//! the source decimal mark to the document mark, appends columns on the right
//! when needed and records the whole paste as one undo step.

use std::collections::BTreeSet;
use std::sync::Arc;

use tabtool_core::{CsvError, Locale, convert_decimal_mark, make_tsv, parse_tsv};
use tabtool_large::LargeFile;

use crate::Document;
use crate::types::{DocumentNotice, GridSelection};
use crate::undo::UndoAction;

impl Document {
    /// Rows selected in visual column order, or columns selected across all
    /// rows, as tab-separated text. Returns `None` when nothing is selected.
    ///
    /// Numeric cells use `decimal_separator`; text is copied as is, with tabs
    /// and line breaks inside cells flattened. The locale is accepted for API
    /// parity with the callers that also sort; the separator is supplied
    /// explicitly because the document layer has no locale decimal data.
    /// A column selection copies only the visible rows (the applied filter's
    /// snapshot, or every row without a filter), never hidden rows.
    pub fn copy_selection(&self, locale: &Locale, decimal_separator: &str) -> Option<String> {
        let source = self.row_source()?;

        if !self.selection.rows.is_empty() {
            return self.copy_rows(&self.selection.rows, locale, decimal_separator);
        }
        if self.selection.columns.is_empty() {
            return None;
        }

        let column_count = source.column_count();
        let selected: Vec<usize> = self
            .selection
            .columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        if selected.is_empty() {
            return None;
        }
        let visible_rows: Vec<usize> = match self.row_filter.visible_rows.as_ref() {
            Some(visible_rows) => visible_rows.clone(),
            None => (0..source.row_count()).collect(),
        };
        if !self.fits_copy_limit(visible_rows.len().saturating_mul(selected.len())) {
            return None;
        }

        let rows: Vec<Vec<String>> = visible_rows
            .iter()
            .map(|&row| {
                selected
                    .iter()
                    .map(|&column| source.cell(row, column))
                    .collect()
            })
            .collect();
        Some(make_tsv(
            &rows,
            &self.configuration.decimal_mark,
            decimal_separator,
        ))
    }

    /// The given rows as TSV in visual column order.
    ///
    /// Returns `None` when no requested row exists. A large document also
    /// returns `None` (and raises [`DocumentNotice::CopyLimitExceeded`]) when
    /// the copy would exceed [`LargeFile::MAX_COPY_CELLS`]; an in-memory
    /// document has no such limit.
    pub fn copy_rows(
        &self,
        rows: &BTreeSet<usize>,
        _locale: &Locale,
        decimal_separator: &str,
    ) -> Option<String> {
        let source = self.row_source()?;
        let row_count = source.row_count();
        let column_count = source.column_count();
        let selected: Vec<usize> = rows
            .iter()
            .copied()
            .filter(|&row| row < row_count)
            .collect();
        if selected.is_empty() || !self.fits_copy_limit(selected.len().saturating_mul(column_count))
        {
            return None;
        }

        let values: Vec<Vec<String>> = selected
            .iter()
            .map(|&row| {
                (0..column_count)
                    .map(|column| source.cell(row, column))
                    .collect()
            })
            .collect();
        Some(make_tsv(
            &values,
            &self.configuration.decimal_mark,
            decimal_separator,
        ))
    }

    /// Copies the given large-file selection as tab-separated text.
    ///
    /// Selected rows win over selected columns, as in `copySelection`. Rows
    /// copy every column of each requested row; columns copy the requested
    /// columns across the visible rows (a large document cannot apply a filter,
    /// but a visible-row set is honored if one exists). Out-of-range indexes
    /// are ignored.
    ///
    /// Returns [`CsvError::RowNotEditable`] when there is no large source,
    /// nothing valid is selected, or the copy would exceed
    /// [`LargeFile::MAX_COPY_CELLS`]; the last case also raises
    /// [`DocumentNotice::CopyLimitExceeded`].
    pub fn copy_large_selection(
        &self,
        rows: &BTreeSet<usize>,
        columns: &BTreeSet<usize>,
        decimal_separator: &str,
    ) -> Result<String, CsvError> {
        let Some(large) = &self.large else {
            return Err(CsvError::RowNotEditable);
        };
        let source = large.source.as_ref();

        if !rows.is_empty() {
            let selected: Vec<usize> = rows
                .iter()
                .copied()
                .filter(|&row| row < source.row_count())
                .collect();
            if selected.is_empty()
                || !self.fits_copy_limit(selected.len().saturating_mul(source.column_count()))
            {
                return Err(CsvError::RowNotEditable);
            }
            let values: Vec<Vec<String>> = selected
                .iter()
                .map(|&row| {
                    (0..source.column_count())
                        .map(|column| source.cell(row, column))
                        .collect()
                })
                .collect();
            return Ok(make_tsv(
                &values,
                &self.configuration.decimal_mark,
                decimal_separator,
            ));
        }

        let column_count = source.column_count();
        let selected: Vec<usize> = columns
            .iter()
            .copied()
            .filter(|&column| column < column_count)
            .collect();
        if selected.is_empty() {
            return Err(CsvError::RowNotEditable);
        }
        let visible_rows: Vec<usize> = match &self.row_filter.visible_rows {
            Some(visible_rows) => visible_rows.clone(),
            None => (0..source.row_count()).collect(),
        };
        if !self.fits_copy_limit(visible_rows.len().saturating_mul(selected.len())) {
            return Err(CsvError::RowNotEditable);
        }
        let values: Vec<Vec<String>> = visible_rows
            .iter()
            .map(|&row| {
                selected
                    .iter()
                    .map(|&column| source.cell(row, column))
                    .collect()
            })
            .collect();
        Ok(make_tsv(
            &values,
            &self.configuration.decimal_mark,
            decimal_separator,
        ))
    }

    /// Inserts a TSV block after the last selected row (or at the end).
    ///
    /// Extra fields add columns on the right. Numeric cells are converted from
    /// `decimal_separator` to the document decimal mark. The inserted rows
    /// become the selection and the whole paste is one undo operation.
    pub fn paste_tsv(&mut self, text: &str, _locale: &Locale, decimal_separator: &str) {
        if !self.can_edit_content() || !self.can_edit_row_structure() {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };

        let parsed_rows = parse_tsv(text);
        let field_count = parsed_rows.iter().map(Vec::len).max().unwrap_or(0);
        if field_count == 0 {
            return;
        }

        let row_count = current.row_count();
        let insertion = self
            .selection
            .rows
            .last()
            .map_or(row_count, |&row| row.saturating_add(1))
            .min(row_count);
        let document_mark = self.configuration.decimal_mark.clone();
        let header = self.configuration.first_row_as_header.then(String::new);

        let mut updated = Arc::clone(&current);
        {
            let table = Arc::make_mut(&mut updated);
            for _ in table.columns.len()..field_count {
                table.add_column(table.columns.len(), header.as_deref());
            }
            for offset in 0..parsed_rows.len() {
                table.add_row(insertion + offset);
            }
            for (row_offset, fields) in parsed_rows.iter().enumerate() {
                for (column, field) in fields.iter().enumerate() {
                    let value = convert_decimal_mark(field, decimal_separator, &document_mark);
                    table.set_cell(value, insertion + row_offset, column);
                }
            }
        }

        self.replace_table_with(UndoAction::PasteData, updated, None);
        self.selection = GridSelection {
            rows: (insertion..insertion + parsed_rows.len()).collect(),
            columns: BTreeSet::new(),
        };
    }

    /// Whether a copy of `cells` cells is allowed; a refused copy raises a
    /// notice.
    ///
    /// The limit protects the mapped-file reader: an in-memory document already
    /// holds its cells and copies without the large-file cap, matching
    /// `CSVDocumentClipboard`'s `largeSource != nil` check.
    fn fits_copy_limit(&self, cells: usize) -> bool {
        if self.large.is_none() || cells <= LargeFile::MAX_COPY_CELLS {
            return true;
        }
        self.set_notice(DocumentNotice::CopyLimitExceeded);
        false
    }
}

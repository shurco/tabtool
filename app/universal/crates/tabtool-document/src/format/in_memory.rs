//! Decimal-mark and header changes that transform the in-memory table.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::Document;
use crate::undo::UndoAction;

impl Document {
    /// Converts numeric cells from the current decimal mark to `mark`.
    ///
    /// Only numeric text changes; formatting such as leading zeros and
    /// exponents is preserved. The configuration follows the table so saving
    /// and pasting use the new mark. One undo snapshot covers both.
    pub fn set_decimal_mark(&mut self, mark: &str) {
        if !self.can_edit_content() || mark.is_empty() || mark == self.configuration.decimal_mark {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let from = self.configuration.decimal_mark.clone();
        let mut updated = Arc::clone(&current);
        Arc::make_mut(&mut updated).convert_decimal_mark(&from, mark);

        let mut configuration = self.configuration.clone();
        configuration.decimal_mark = mark.to_owned();
        let includes_utf8_bom = self.had_utf8_bom;
        self.apply_content_change(
            updated,
            configuration,
            includes_utf8_bom,
            UndoAction::ChangeFormat,
        );
    }

    /// Turns header mode on or off by moving the first row across the header
    /// boundary, updating both the table and the configuration.
    ///
    /// A large document routes to [`Document::set_large_header_enabled`], which
    /// moves the source's row mapping instead of a table. One undo snapshot
    /// restores the table, configuration and first row, or the large row
    /// mapping and configuration.
    pub fn set_header_enabled(&mut self, enabled: bool) {
        if self.is_large_mode() {
            self.set_large_header_enabled(enabled);
            return;
        }
        if !self.can_edit_content() {
            return;
        }
        let Some(current) = self.table.as_ref().map(Arc::clone) else {
            return;
        };
        let has_headers = current.headers().is_some();

        if has_headers == enabled {
            if self.configuration.first_row_as_header == enabled {
                return;
            }
            let mut configuration = self.configuration.clone();
            configuration.first_row_as_header = enabled;
            let includes_utf8_bom = self.had_utf8_bom;
            self.apply_content_change(
                current,
                configuration,
                includes_utf8_bom,
                UndoAction::ToggleHeader,
            );
            return;
        }

        let mut updated = Arc::clone(&current);
        if enabled {
            let headers: Vec<String> = (0..current.columns.len())
                .map(|column| current.cell(0, column))
                .collect();
            let table = Arc::make_mut(&mut updated);
            table.set_headers(Some(headers));
            table.delete_rows(&BTreeSet::from([0]));
        } else if let Some(headers) = current.headers() {
            let table = Arc::make_mut(&mut updated);
            table.set_headers(None);
            table.add_row(0);
            for (column, header) in headers.iter().enumerate() {
                table.set_cell(header.clone(), 0, column);
            }
        }

        let mut configuration = self.configuration.clone();
        configuration.first_row_as_header = enabled;
        let includes_utf8_bom = self.had_utf8_bom;
        self.apply_content_change(
            updated,
            configuration,
            includes_utf8_bom,
            UndoAction::ToggleHeader,
        );
    }
}

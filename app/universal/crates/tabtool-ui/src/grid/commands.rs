//! Applying a [`GridAction`] to a [`Document`].
//!
//! The grid never mutates the document; it returns actions, and this module is
//! the single place that maps them onto the document API. Clipboard actions go
//! through a [`Clipboard`] supplied by the app, so the UI crate stays free of
//! platform clipboard code.
//!
//! Rows in [`GridAction`] are indexes of the source the grid rendered. A grid
//! over a filtered presenter therefore needs its caller to translate display
//! rows into model rows before dispatch; an unfiltered grid passes them
//! through.

use std::collections::BTreeSet;

use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, DocumentPhase, GridSelection};

use super::GridAction;
use super::clipboard::Clipboard;

/// Applies `action` to `document`.
///
/// Returns [`DocumentError::NotReady`] when the document has no rows or columns
/// to act on. A command the document refuses for its own reasons (a filtered
/// row structure, a large file, a copy limit) is a no-op here and reports
/// through the document's state or [`tabtool_document::DocumentNotice`].
pub fn apply_action(
    document: &mut Document,
    action: GridAction,
    locale: &Locale,
    decimal_separator: &str,
    clipboard: &mut dyn Clipboard,
) -> Result<(), DocumentError> {
    if document.row_source().is_none()
        || matches!(
            document.phase(),
            DocumentPhase::Loading | DocumentPhase::ReadFailed(_) | DocumentPhase::UnreadableFile
        )
    {
        return Err(DocumentError::NotReady);
    }

    match action {
        GridAction::CommitCell { row, column, text } => {
            document.set_cell(row, column, &text);
        }
        GridAction::AddRowAbove => document.add_row_above_selection(),
        GridAction::AddRowBelow => document.add_row_below_selection(),
        GridAction::AddColumnLeft => document.add_column_left_of_selection(),
        GridAction::AddColumnRight => document.add_column_right_of_selection(),
        GridAction::DeleteSelection => document.delete_selection(),
        GridAction::MoveRows { from, to } => move_rows(document, &from, to),
        GridAction::MoveColumn { from, to } => move_column(document, from, to),
        GridAction::Sort { column, direction } => {
            document.sort_rows(column, direction, locale);
        }
        GridAction::RemoveSort => document.remove_sort(),
        GridAction::RenameColumn { column, name } => document.rename_column(column, &name),
        GridAction::Copy => {
            if let Some(text) = document.copy_selection(locale, decimal_separator) {
                clipboard.copy(&text);
            }
        }
        GridAction::Paste => {
            if let Some(text) = clipboard.paste() {
                document.paste_tsv(&text, locale, decimal_separator);
            }
        }
        GridAction::SelectAll => select_all(document),
    }
    Ok(())
}

fn select_all(document: &mut Document) {
    let Some(source) = document.row_source() else {
        return;
    };
    let rows: BTreeSet<usize> = match document.row_filter().visible_rows.as_ref() {
        Some(visible) => visible
            .iter()
            .copied()
            .filter(|row| *row < source.row_count())
            .collect(),
        None => (0..source.row_count()).collect(),
    };
    *document.selection_mut() = GridSelection {
        rows,
        columns: BTreeSet::new(),
    };
}

fn move_rows(document: &mut Document, from: &BTreeSet<usize>, to: usize) {
    let Some(source) = document.row_source() else {
        return;
    };
    let row_count = source.row_count();
    let revision = document.revision();
    document.move_rows(from, to);
    if document.revision() == revision {
        return;
    }
    let selected = from.iter().filter(|row| **row < row_count).count();
    let destination = to.min(row_count);
    let moved = destination.saturating_sub(from.iter().filter(|row| **row < destination).count());
    *document.selection_mut() = GridSelection {
        rows: (moved..moved.saturating_add(selected)).collect(),
        columns: BTreeSet::new(),
    };
}

fn move_column(document: &mut Document, from: usize, to: usize) {
    let Some(source) = document.row_source() else {
        return;
    };
    let column_count = source.column_count();
    let selected = document.selection().columns.contains(&from);
    let revision = document.revision();
    document.move_columns(&BTreeSet::from([from]), to);
    if !selected || document.revision() == revision || column_count == 0 {
        return;
    }
    let moved = if to > from { to.saturating_sub(1) } else { to };
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::new(),
        columns: [moved.min(column_count - 1)].into_iter().collect(),
    };
}

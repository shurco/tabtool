//! Applying a [`GridAction`] to a [`Document`].
//!
//! The grid never mutates the document; it returns actions, and this module is
//! the single place that maps them onto the document API. Clipboard actions go
//! through a [`Clipboard`] supplied by the app, so the UI crate stays free of
//! platform clipboard code.
//!
//! Rows in [`GridAction`] are indexes of the source the grid rendered. This
//! dispatcher applies them as-is; the document window translates display rows
//! to model rows before calling it when a filter is active.

use std::collections::BTreeSet;

use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, DocumentPhase, GridSelection};

use super::GridAction;
use super::clipboard::Clipboard;

/// What an action asks the caller to do next.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionOutcome {
    /// The document was updated (or the command was a legitimate no-op).
    Applied,
    /// Paste had no text because the platform has not delivered it yet.
    ///
    /// egui cannot read the clipboard synchronously: a menu-initiated paste
    /// requests [`egui::ViewportCommand::RequestPaste`] and retries the action
    /// when the `Event::Paste` arrives. Keyboard paste usually supplies the
    /// text in the same frame and returns [`Self::Applied`].
    NeedsPasteText,
}

/// Applies `action` to `document`.
///
/// Returns [`DocumentError::NotReady`] when the document has no rows or columns
/// to act on. A command the document refuses for its own reasons (a filtered
/// row structure, a large file, a copy limit) is a no-op here and reports
/// through the document's state or [`tabtool_document::DocumentNotice`].
///
/// Select All is deliberately disabled for large documents: a [`BTreeSet`] of
/// every row would cost hundreds of megabytes at millions of rows, so the
/// caller must offer a range-aware selection first (design §14.18, approval
/// pending).
pub fn apply_action(
    document: &mut Document,
    action: GridAction,
    locale: &Locale,
    decimal_separator: &str,
    clipboard: &mut dyn Clipboard,
) -> Result<ActionOutcome, DocumentError> {
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
            let Some(text) = clipboard.paste() else {
                return Ok(ActionOutcome::NeedsPasteText);
            };
            document.paste_tsv(&text, locale, decimal_separator);
        }
        GridAction::SelectAll => {
            if !document.can_select_all() {
                // Deliberate difference: a large file has no compact way to
                // hold "all rows" in the current selection type.
                return Ok(ActionOutcome::Applied);
            }
            select_all(document);
        }
    }
    Ok(ActionOutcome::Applied)
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

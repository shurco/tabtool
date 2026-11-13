//! Applying grid actions to the document through the display/model mapping.
//!
//! The grid works in display rows and never touches the document. This module
//! translates a filtered grid's display rows to model rows before dispatch,
//! applies the action, and maps the resulting selection and focus back to
//! display rows so painting and keyboard navigation agree with the filter.
//!
//! The same conversions keep the grid selection and the document selection in
//! step for menu commands and the toolbar. They live here, next to the grid
//! composition, so every caller uses one mapping.

use std::collections::BTreeSet;

use crate::grid::{
    ActionOutcome, Clipboard, GridAction, GridFocus, GridSelection, GridState, apply_action,
};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, GridSelection as DocumentSelection};

/// Applies one grid action to `document` and mirrors selection and focus back
/// to `state`.
pub fn apply_grid_action(
    document: &mut Document,
    state: &mut GridState,
    action: GridAction,
    clipboard: &mut dyn Clipboard,
    locale: &Locale,
    decimal_separator: &str,
) -> Result<ActionOutcome, DocumentError> {
    let display_selection = state.selection.clone();
    *document.selection_mut() = DocumentSelection {
        rows: display_rows_to_model(document, &display_selection.rows),
        columns: display_selection.columns,
    };

    let focus = state.focus;
    let focus_model = focus.and_then(|focus| display_row_to_model(document, focus.row));
    let focus_column_id = focus.and_then(|focus| {
        document
            .row_source()
            .and_then(|source| source.column_ids().get(focus.column).copied())
    });
    let moved_rows = matches!(&action, GridAction::MoveRows { .. });

    let Some(action) = map_action(document, action) else {
        return Ok(ActionOutcome::Applied);
    };

    let result = apply_action(document, action, locale, decimal_separator, clipboard);

    let selection = document.selection().clone();
    let display_rows = model_rows_to_display(document, &selection.rows);
    state.selection = GridSelection {
        rows: display_rows.clone(),
        columns: selection.columns,
    };
    state.anchor = None;
    if let Some(focus) = focus {
        let column_count = document
            .row_source()
            .map_or(0, |source| source.column_count());
        let row = if moved_rows {
            display_rows.first().copied()
        } else {
            focus_model
                .and_then(|row| model_row_to_display(document, row))
                .or_else(|| display_rows.first().copied())
        };
        let column = focus_column_id
            .and_then(|id| {
                document.row_source().and_then(|source| {
                    source
                        .column_ids()
                        .iter()
                        .position(|candidate| *candidate == id)
                })
            })
            .unwrap_or(focus.column);
        let focus = if column_count == 0 {
            None
        } else {
            row.map(|row| GridFocus::new(row, column.min(column_count - 1)))
        };
        state.focus_cell(focus);
    }
    result
}

/// Copies the document selection into the grid as display rows.
///
/// A menu command that changes the document selection calls this so the grid
/// paints and navigates the same rows.
pub fn sync_selection_to_grid(document: &Document, state: &mut GridState) {
    let selection = document.selection();
    let mapped = GridSelection {
        rows: model_rows_to_display(document, &selection.rows),
        columns: selection.columns.clone(),
    };
    if state.selection != mapped {
        state.selection = mapped;
        state.anchor = None;
    }
}

/// Copies the grid selection into the document as model rows.
///
/// The view calls this after drawing so toolbar enablement and menu commands
/// see the rows the user clicked.
pub fn sync_selection_to_document(document: &mut Document, state: &GridState) {
    let mapped = DocumentSelection {
        rows: display_rows_to_model(document, &state.selection.rows),
        columns: state.selection.columns.clone(),
    };
    if document.selection() != &mapped {
        *document.selection_mut() = mapped;
    }
}

fn map_action(document: &Document, action: GridAction) -> Option<GridAction> {
    Some(match action {
        GridAction::CommitCell { row, column, text } => GridAction::CommitCell {
            row: display_row_to_model(document, row)?,
            column,
            text,
        },
        GridAction::MoveRows { from, to } => GridAction::MoveRows {
            from: display_rows_to_model(document, &from),
            to: display_insertion_to_model(document, to),
        },
        other => other,
    })
}

fn row_count(document: &Document) -> usize {
    document.row_source().map_or(0, |source| source.row_count())
}

fn visible_rows(document: &Document) -> Option<&[usize]> {
    document.row_filter().visible_rows.as_deref()
}

fn display_row_to_model(document: &Document, row: usize) -> Option<usize> {
    let rows = row_count(document);
    match visible_rows(document) {
        Some(visible) => visible.get(row).copied().filter(|model| *model < rows),
        None => (row < rows).then_some(row),
    }
}

fn model_row_to_display(document: &Document, row: usize) -> Option<usize> {
    match visible_rows(document) {
        Some(visible) => visible.binary_search(&row).ok(),
        None => (row < row_count(document)).then_some(row),
    }
}

fn display_rows_to_model(document: &Document, rows: &BTreeSet<usize>) -> BTreeSet<usize> {
    rows.iter()
        .filter_map(|row| display_row_to_model(document, *row))
        .collect()
}

fn model_rows_to_display(document: &Document, rows: &BTreeSet<usize>) -> BTreeSet<usize> {
    rows.iter()
        .filter_map(|row| model_row_to_display(document, *row))
        .collect()
}

fn display_insertion_to_model(document: &Document, slot: usize) -> usize {
    match visible_rows(document) {
        Some(visible) => visible
            .get(slot)
            .copied()
            .unwrap_or_else(|| row_count(document)),
        None => slot.min(row_count(document)),
    }
}

//! Grid action dispatch and the egui clipboard for a document window.
//!
//! The grid works in display rows and never touches the document. This module
//! translates a filtered grid's display rows to model rows before dispatch,
//! applies the action, and maps the resulting selection and focus back to
//! display rows so painting and keyboard navigation agree with the filter.

use std::collections::BTreeSet;

use egui::{Context, Event};
use tabtool_document::{Document, DocumentError, GridSelection as DocumentSelection};
use tabtool_ui::grid::{
    ActionOutcome, Clipboard, GridAction, GridFocus, GridSelection, apply_action,
};

use super::{DocumentWindow, WindowState};

/// The platform clipboard as egui exposes it.
///
/// Copy is queued as an egui output command. Paste arrives as an
/// [`Event::Paste`] from the backend; the event is taken so a text field cannot
/// paste the same text twice. A menu-initiated paste has no event yet: the
/// adapter requests the platform paste and reports
/// [`ActionOutcome::NeedsPasteText`], so the app can retry when the event
/// arrives.
pub struct EguiClipboard<'a> {
    ctx: &'a Context,
    paste: Option<String>,
}

impl<'a> EguiClipboard<'a> {
    /// Wraps this frame's clipboard events.
    pub fn new(ctx: &'a Context) -> Self {
        Self {
            ctx,
            paste: take_paste(ctx),
        }
    }
}

impl Clipboard for EguiClipboard<'_> {
    fn copy(&mut self, text: &str) {
        self.ctx.copy_text(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        if self.paste.is_none() {
            // A menu-initiated paste has no Event::Paste yet. Ask the backend
            // for it; Task 6 retries the Paste action when the event arrives.
            self.ctx
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        self.paste.take()
    }
}

fn take_paste(ctx: &Context) -> Option<String> {
    ctx.input_mut(|input| {
        let index = input
            .events
            .iter()
            .position(|event| matches!(event, Event::Paste(_)))?;
        match input.events.remove(index) {
            Event::Paste(text) => Some(text),
            _ => None,
        }
    })
}

impl DocumentWindow {
    /// Applies one grid action to the document.
    ///
    /// The grid selection and action rows are display rows. They are translated
    /// through the document's visible-row filter before dispatch, and the
    /// resulting model selection and focus are translated back to display
    /// rows.
    pub fn apply_grid_action(
        &self,
        action: GridAction,
        clipboard: &mut dyn Clipboard,
    ) -> Result<ActionOutcome, DocumentError> {
        let mut state = self.lock();
        apply_to_window(&mut state, action, clipboard)
    }

    /// The grid's row/column selection in display rows.
    pub fn grid_selection(&self) -> GridSelection {
        self.lock().grid.selection.clone()
    }

    /// Replaces the grid selection with display rows and columns.
    pub fn set_grid_selection(&self, selection: GridSelection) {
        let mut state = self.lock();
        state.grid.selection = selection;
        state.grid.anchor = None;
    }

    /// The focused grid cell in display coordinates.
    pub fn grid_focus(&self) -> Option<GridFocus> {
        self.lock().grid.focus
    }

    /// Sets the focused grid cell and requests keyboard focus for it.
    pub fn set_grid_focus(&self, focus: Option<GridFocus>) {
        self.lock().grid.focus_cell(focus);
    }

    /// Takes the document's one-shot notice for the window to present.
    pub fn take_notice(&self) -> Option<tabtool_document::DocumentNotice> {
        self.lock().document.take_notice()
    }
}

fn apply_to_window(
    state: &mut WindowState,
    action: GridAction,
    clipboard: &mut dyn Clipboard,
) -> Result<ActionOutcome, DocumentError> {
    let display_selection = state.grid.selection.clone();
    *state.document.selection_mut() = DocumentSelection {
        rows: display_rows_to_model(&state.document, &display_selection.rows),
        columns: display_selection.columns,
    };

    let focus = state.grid.focus;
    let focus_model = focus.and_then(|focus| display_row_to_model(&state.document, focus.row));
    let focus_column_id = focus.and_then(|focus| {
        state
            .document
            .row_source()
            .and_then(|source| source.column_ids().get(focus.column).copied())
    });
    let moved_rows = matches!(&action, GridAction::MoveRows { .. });

    let Some(action) = map_action(&state.document, action) else {
        return Ok(ActionOutcome::Applied);
    };

    let locale = state.locale.clone();
    let decimal_separator = state.decimal_separator.clone();
    let result = apply_action(
        &mut state.document,
        action,
        &locale,
        &decimal_separator,
        clipboard,
    );

    let selection = state.document.selection().clone();
    let display_rows = model_rows_to_display(&state.document, &selection.rows);
    state.grid.selection = GridSelection {
        rows: display_rows.clone(),
        columns: selection.columns,
    };
    state.grid.anchor = None;
    if let Some(focus) = focus {
        let column_count = state
            .document
            .row_source()
            .map_or(0, |source| source.column_count());
        let row = if moved_rows {
            display_rows.first().copied()
        } else {
            focus_model
                .and_then(|row| model_row_to_display(&state.document, row))
                .or_else(|| display_rows.first().copied())
        };
        let column = focus_column_id
            .and_then(|id| {
                state.document.row_source().and_then(|source| {
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
        state.grid.focus_cell(focus);
    }
    result
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

#[cfg(test)]
mod tests {
    use egui::{Context, Event, OutputCommand};
    use tabtool_ui::grid::Clipboard;

    use super::EguiClipboard;

    #[test]
    fn paste_reads_and_consumes_the_event() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput {
            events: vec![Event::Paste("x\ty".to_owned())],
            ..Default::default()
        });
        let mut clipboard = EguiClipboard::new(&ctx);

        assert_eq!(clipboard.paste().as_deref(), Some("x\ty"));
        assert_eq!(clipboard.paste(), None);
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn copy_queues_an_egui_output_command() {
        let ctx = Context::default();
        ctx.begin_pass(egui::RawInput::default());
        EguiClipboard::new(&ctx).copy("a\tb");
        let commands = ctx.output(|output| output.commands.clone());
        let mut output = ctx.end_pass();
        output.textures_delta.clear();

        assert!(matches!(
            commands.first(),
            Some(OutputCommand::CopyText(text)) if text == "a\tb"
        ));
    }
}

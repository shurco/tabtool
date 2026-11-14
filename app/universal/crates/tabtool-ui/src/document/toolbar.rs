//! The document toolbar above the grid.

use egui::Ui;
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use crate::grid::{GridAction, GridState};
use crate::widgets::toolbar;
use crate::widgets::{ButtonKind, PushButton, toolbar_separator};

use super::{can_export, enabled_filter_count};

/// What the toolbar asked for.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ToolbarResponse {
    /// Structural commands to apply through the display-to-model dispatcher.
    pub actions: Vec<GridAction>,
    /// Export was pressed; the window opens the export panel.
    pub export: bool,
}

/// Draws the toolbar and returns the commands it requested.
///
/// The filter toggle changes the document directly because it is a view flag,
/// not a table edit. Every other button is a [`GridAction`] the caller applies
/// through the display-to-model dispatcher.
pub fn show(
    ui: &mut Ui,
    document: &mut Document,
    state: &GridState,
    language: Language,
) -> ToolbarResponse {
    let mut actions = Vec::new();
    let mut export = false;
    let editable = document.can_edit_structure();
    let can_insert_rows = document.can_insert_rows();
    let row_structure = document.can_edit_row_structure();
    let selection = &state.selection;
    let source = document.row_source();
    let row_count = source.map_or(0, |source| source.row_count());
    let column_count = source.map_or(0, |source| source.column_count());
    let selected_rows = selection.rows.iter().any(|row| *row < row_count);
    let selected_columns = selection
        .columns
        .iter()
        .any(|column| *column < column_count);

    let add_above = can_insert_rows && selected_rows;
    let add_below = can_insert_rows;
    let add_left = editable && selected_columns;
    let add_right = editable;
    let delete = if !editable {
        false
    } else if selected_rows {
        row_structure
    } else {
        selected_columns && column_count > 1
    };
    let export_enabled = can_export(document);
    let filter_enabled = document.can_filter();
    let filter_visible = document.row_filter().is_bar_visible;
    let filter_count = enabled_filter_count(document);

    toolbar(ui, |ui| {
        if tool_button(ui, "\u{2191}+", tr(language, "Add Row Above"), add_above).clicked() {
            actions.push(GridAction::AddRowAbove);
        }
        if tool_button(ui, "\u{2193}+", tr(language, "Add Row Below"), add_below).clicked() {
            actions.push(GridAction::AddRowBelow);
        }
        toolbar_separator(ui);
        if tool_button(ui, "\u{2190}+", tr(language, "Add Column Left"), add_left).clicked() {
            actions.push(GridAction::AddColumnLeft);
        }
        if tool_button(ui, "\u{2192}+", tr(language, "Add Column Right"), add_right).clicked() {
            actions.push(GridAction::AddColumnRight);
        }
        toolbar_separator(ui);
        if tool_button(ui, "\u{2212}", tr(language, "Delete Selection"), delete).clicked() {
            actions.push(GridAction::DeleteSelection);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let filter_label = if filter_visible {
                tr(language, "Hide Filter")
            } else {
                tr(language, "Show Filter")
            };
            let filter_label = if filter_count > 0 {
                format!("{filter_label} ({filter_count})")
            } else {
                filter_label.to_owned()
            };
            if PushButton::new(&filter_label, ButtonKind::Plain)
                .selected(filter_visible)
                .enabled(filter_enabled)
                .show(ui)
                .clicked()
            {
                document.toggle_filter_bar();
            }
            if PushButton::new(tr(language, "Export"), ButtonKind::Plain)
                .enabled(export_enabled)
                .show(ui)
                .clicked()
            {
                export = true;
            }
        });
    });
    ToolbarResponse { actions, export }
}

fn tool_button(ui: &mut Ui, glyph: &str, name: &str, enabled: bool) -> egui::Response {
    PushButton::new(glyph, ButtonKind::Plain)
        .enabled(enabled)
        .accessible_name(name)
        .show(ui)
        .on_hover_text(name)
}

//! Drawing the document content: the grid or its loading placeholder.

use egui::{Align, Layout, RichText, Ui};
use tabtool_document::{Document, DocumentPhase, LargeStatus};
use tabtool_i18n::{Language, tr};

use crate::document::progress::{self, saving_text};
use crate::document::{DocumentView, error_overlay};
use crate::grid::{
    GridAction, GridAppearance, GridMenuOptions, GridOptions, GridPresenter, GridView, SortSpec,
};
use crate::theme::current;

impl DocumentView<'_> {
    pub(in crate::document) fn show_content(&mut self, ui: &mut Ui) -> Vec<GridAction> {
        let phase = *self.document.phase();
        match phase {
            DocumentPhase::ReadFailed(_) | DocumentPhase::UnreadableFile => {
                let can_detect = self.document.file_path().is_some() && !self.is_busy();
                if error_overlay::show(ui, self.language, &phase, can_detect) {
                    self.detect_format();
                }
                Vec::new()
            }
            DocumentPhase::Loading => {
                centered_text(ui, tr(self.language, "Loading…"));
                Vec::new()
            }
            DocumentPhase::Ready | DocumentPhase::NeedsAttention(_) => {
                if self.document.has_content() {
                    self.show_grid(ui)
                } else {
                    let status_fraction = match self.document.large_status() {
                        LargeStatus::Saving(fraction) => Some(fraction),
                        _ => None,
                    };
                    if let Some(fraction) = self.document.large_save_progress().or(status_fraction)
                    {
                        centered_progress(ui, self.language, fraction);
                    } else {
                        centered_text(ui, tr(self.language, "Loading…"));
                    }
                    Vec::new()
                }
            }
        }
    }

    pub(in crate::document) fn show_grid(&mut self, ui: &mut Ui) -> Vec<GridAction> {
        let Some(source) = self.document.row_source() else {
            return Vec::new();
        };
        let options = grid_options(
            self.document,
            self.language,
            self.appearance,
            !self.input_blocked,
        );
        let visible = self.document.row_filter().visible_rows.as_deref();
        let presenter = GridPresenter::new(source, visible);
        GridView {
            source: &presenter,
            state: &mut *self.state,
            options,
        }
        .show(ui)
    }
}

/// The grid options for `document`: sort arrow, decimal mark and capabilities.
///
/// The desktop window and the touch editor build on this so a large file's
/// header offset, a filtered view's menu limits and the document's editable
/// state stay in one place. Callers add their own row height and input mode;
/// the touch editor uses [`crate::grid::GridOptions::for_touch`].
pub fn grid_options(
    document: &Document,
    language: Language,
    appearance: GridAppearance,
    keys_enabled: bool,
) -> GridOptions {
    let sort = document.active_sort().and_then(|sort| {
        document.row_source().and_then(|source| {
            source
                .column_ids()
                .iter()
                .position(|column| *column == sort.column_id)
                .map(|column| SortSpec {
                    column,
                    direction: sort.direction,
                })
        })
    });
    let header_offset = if document.is_large_mode() && document.configuration().first_row_as_header
    {
        1
    } else {
        0
    };
    GridOptions {
        row_height: appearance.row_height,
        alternating_rows: appearance.alternating_rows,
        shows_row_numbers: appearance.shows_row_numbers,
        decimal_mark: document.configuration().decimal_mark.clone(),
        editable: document.can_edit_cells(),
        sort,
        language,
        header_offset,
        menu: GridMenuOptions::of(document),
        keys_enabled,
        ..GridOptions::default()
    }
}

fn centered_text(ui: &mut Ui, text: &str) {
    let theme = current(ui.ctx());
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(theme.metrics.window_padding * 3.0);
        ui.label(
            RichText::new(text)
                .size(theme.typography.body)
                .color(theme.colors.secondary_label),
        );
    });
}

fn centered_progress(ui: &mut Ui, language: Language, fraction: f64) {
    let theme = current(ui.ctx());
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(theme.metrics.window_padding * 3.0);
        ui.label(
            RichText::new(saving_text(language, fraction))
                .size(theme.typography.body)
                .color(theme.colors.secondary_label),
        );
        progress::progress_bar(ui, fraction, 160.0);
    });
}

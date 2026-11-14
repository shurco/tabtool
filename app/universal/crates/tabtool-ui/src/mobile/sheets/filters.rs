//! The Filters sheet: criterion cards, the footer actions and the draft.

use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, Ui};
use tabtool_core::{CriterionId, FilterOperator, Locale};
use tabtool_document::Document;
use tabtool_i18n::tr;

use super::{
    PickerTouch, SheetScreenResponse, SheetStyle, touch_button, touch_checkbox, touch_field,
    touch_picker_sized,
};
use crate::document::rows_text;
use crate::filter::FilterAction;
use crate::filter::bar::{column_options, operator_title};
use crate::mobile::{
    SheetOptions, TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest, sheet,
};
use crate::theme::current;
use crate::widgets::ButtonKind;

/// The Filters sheet's native text session.
#[derive(Default)]
pub struct FiltersSheet {
    editing: Option<CriterionId>,
    pending_action: Option<FilterAction>,
    pending_close: bool,
}

impl FiltersSheet {
    /// A sheet with no open value field.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the sheet and edits the document's draft criteria.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &mut Document,
        text_input: &mut dyn TouchTextInput,
        locale: &Locale,
    ) -> SheetScreenResponse {
        let language = style.language;
        let committed = self.poll_text_input(document, text_input);
        if committed && let Some(action) = self.pending_action.take() {
            self.apply(action, document, text_input);
        }
        if self.pending_close && self.editing.is_none() {
            self.pending_close = false;
            return SheetScreenResponse {
                close_requested: true,
            };
        }

        if !document.can_filter() {
            let response = sheet(
                ui.ctx(),
                "mobile-filters-sheet",
                style.metrics,
                SheetOptions::new(tr(language, "Filters"), tr(language, "Done"))
                    .keyboard_height(style.keyboard_height),
                |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(tr(language, "Not available in large file mode."))
                            .size(style.metrics.body)
                            .color(current(ui.ctx()).colors.secondary_label),
                    );
                },
            );
            return SheetScreenResponse {
                close_requested: response.close_requested,
            };
        }

        let columns = column_options(document);
        if let Some((first, _)) = columns.first() {
            document.row_filter_mut().fill_missing_columns(*first);
        }
        if document.row_filter().criteria.is_empty() {
            document.row_filter_mut().add_criterion(0);
        }
        let names: Vec<String> = columns.iter().map(|(_, name)| name.clone()).collect();
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut criteria = std::mem::take(&mut document.row_filter_mut().criteria);
        let mut action = None;
        let mut open_field = None;

        let response = sheet(
            ui.ctx(),
            "mobile-filters-sheet",
            style.metrics,
            SheetOptions::new(tr(language, "Filters"), tr(language, "Done"))
                .keyboard_height(style.keyboard_height),
            |ui| {
                for criterion in &mut criteria {
                    let theme = current(ui.ctx());
                    Frame::new()
                        .fill(theme.colors.header_background)
                        .corner_radius(CornerRadius::same(theme.metrics.large_corner_radius))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let mut enabled = criterion.is_enabled;
                                if touch_checkbox(
                                    ui,
                                    style,
                                    tr(language, "Enable Criterion"),
                                    &mut enabled,
                                    true,
                                )
                                .changed()
                                {
                                    criterion.is_enabled = enabled;
                                }

                                let selected = criterion
                                    .column_id
                                    .and_then(|id| {
                                        columns.iter().position(|(column, _)| *column == id)
                                    })
                                    .unwrap_or(0);
                                let selected = names
                                    .get(selected)
                                    .map_or(tr(language, "Choose Column"), String::as_str);
                                let picker_width =
                                    (ui.available_width() - style.metrics.control_height - 8.0)
                                        .max(80.0);
                                if let Some(index) = touch_picker_sized(
                                    ui,
                                    style,
                                    ("mobile-filter-column", criterion.id),
                                    tr(language, "Column"),
                                    selected,
                                    &name_refs,
                                    PickerTouch::enabled(true).width(picker_width),
                                ) {
                                    criterion.column_id = Some(columns[index].0);
                                }

                                if touch_button(style, "\u{2212}", ButtonKind::Plain)
                                    .accessible_name(tr(language, "Remove Criterion"))
                                    .show(ui)
                                    .clicked()
                                {
                                    action = Some(FilterAction::Remove(criterion.id));
                                }
                            });

                            ui.horizontal(|ui| {
                                let operators: Vec<&str> = FilterOperator::ALL
                                    .iter()
                                    .map(|operator| operator_title(language, *operator))
                                    .collect();
                                let selected = FilterOperator::ALL
                                    .iter()
                                    .position(|operator| *operator == criterion.op)
                                    .unwrap_or(0);
                                let operator_width = 150.0_f32.min(ui.available_width() * 0.5);
                                if let Some(index) = touch_picker_sized(
                                    ui,
                                    style,
                                    ("mobile-filter-operator", criterion.id),
                                    tr(language, "Operator"),
                                    operators[selected],
                                    &operators,
                                    PickerTouch::enabled(true).width(operator_width),
                                ) {
                                    criterion.op = FilterOperator::ALL[index];
                                }

                                let needs_value = criterion.op.needs_value();
                                let field = touch_field(
                                    ui,
                                    style,
                                    tr(language, "Value"),
                                    &criterion.value,
                                    tr(language, "Value"),
                                    needs_value,
                                );
                                if field.clicked() && needs_value {
                                    open_field =
                                        Some((criterion.id, field.rect, criterion.value.clone()));
                                }
                            });

                            if !criterion.is_valid() {
                                let theme = current(ui.ctx());
                                ui.label(
                                    RichText::new(tr(language, "Invalid regular expression"))
                                        .size(style.metrics.caption)
                                        .color(theme.colors.error),
                                );
                            }
                        });
                    ui.add_space(8.0);
                }

                ui.add_space(4.0);
                let invalid = criteria
                    .iter()
                    .any(|criterion| criterion.is_enabled && !criterion.is_valid());
                let total = document.row_source().map_or(0, |source| source.row_count());
                let visible = document.row_filter().visible_rows.as_ref().map(Vec::len);
                ui.horizontal(|ui| {
                    if touch_button(style, tr(language, "Clear"), ButtonKind::Bordered)
                        .show(ui)
                        .clicked()
                    {
                        action = Some(FilterAction::Clear);
                    }
                    ui.label(
                        RichText::new(rows_text(language, total, visible, locale))
                            .size(style.metrics.caption)
                            .color(current(ui.ctx()).colors.secondary_label),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if touch_button(style, tr(language, "Apply"), ButtonKind::Bordered)
                            .enabled(!invalid)
                            .show(ui)
                            .clicked()
                        {
                            action = Some(FilterAction::ApplyAll);
                        }
                        if touch_button(style, "\u{002B}", ButtonKind::Plain)
                            .text_size(style.metrics.body)
                            .accessible_name(tr(language, "Add Criterion"))
                            .show(ui)
                            .clicked()
                        {
                            let after = criteria.last().map(|criterion| criterion.id);
                            if let Some(after) = after {
                                action = Some(FilterAction::Add(after));
                            }
                        }
                    });
                });
                ui.add_space(4.0);
            },
        );

        document.row_filter_mut().criteria = criteria;
        let applied = action.is_some();
        if let Some(action) = action {
            if action.changes_rows() && self.editing.is_some() {
                self.pending_action = Some(action);
                text_input.commit();
            } else {
                self.apply(action, document, text_input);
            }
        }
        if !applied && let Some((id, rect, text)) = open_field {
            self.editing = Some(id);
            text_input.open(TouchTextInputRequest {
                rect,
                text,
                multiline: false,
            });
        }
        if response.close_requested {
            if self.editing.is_some() {
                self.pending_close = true;
                text_input.commit();
                return SheetScreenResponse {
                    close_requested: false,
                };
            }
            self.pending_action = None;
        }

        SheetScreenResponse {
            close_requested: response.close_requested,
        }
    }

    /// Requests a native commit before the host closes the sheet from
    /// navigation.
    ///
    /// Returns whether the sheet can close now. When it returns `false`, the
    /// host must keep presenting the sheet so [`Self::show`] can drain the
    /// commit and then report its close.
    pub fn finish(&mut self, text_input: &mut dyn TouchTextInput) -> bool {
        if self.editing.is_some() {
            self.pending_close = true;
            text_input.commit();
            false
        } else {
            true
        }
    }

    fn apply(
        &mut self,
        action: FilterAction,
        document: &mut Document,
        text_input: &mut dyn TouchTextInput,
    ) {
        match action {
            FilterAction::ApplyAll => document.apply_all_filters(),
            FilterAction::Apply(id) => document.apply_filter(id),
            FilterAction::Clear => {
                document.clear_filter();
                if self.editing.is_some() {
                    self.editing = None;
                    text_input.close();
                }
            }
            FilterAction::Add(id) => document.row_filter_mut().add_criterion(id),
            FilterAction::Remove(id) => {
                document.row_filter_mut().remove_criterion(id);
                if self.editing == Some(id) {
                    self.editing = None;
                    text_input.close();
                }
            }
            FilterAction::Hide => {}
        }
    }

    fn poll_text_input(
        &mut self,
        document: &mut Document,
        text_input: &mut dyn TouchTextInput,
    ) -> bool {
        let Some(id) = self.editing else {
            return false;
        };
        match text_input.poll() {
            Some(TouchTextInputOutcome::Committed(text)) => {
                self.editing = None;
                if let Some(criterion) = document
                    .row_filter_mut()
                    .criteria
                    .iter_mut()
                    .find(|criterion| criterion.id == id)
                {
                    criterion.value = text;
                }
                true
            }
            Some(TouchTextInputOutcome::Cancelled) => {
                self.editing = None;
                self.pending_action = None;
                false
            }
            None if !text_input.is_active() => {
                self.editing = None;
                self.pending_action = None;
                false
            }
            None => false,
        }
    }
}

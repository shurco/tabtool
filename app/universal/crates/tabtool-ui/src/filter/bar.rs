//! The inline filter bar: one row per criterion and a footer of bar actions.

use egui::{Align, CornerRadius, Key, Layout, Modifiers, RichText, Stroke, StrokeKind, Ui, Vec2};
use tabtool_core::{ColumnId, CriterionId, FilterOperator};
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use crate::grid::GridPresenter;
use crate::theme::current;
use crate::widgets::{ButtonKind, Picker, PushButton, text_field_sized, toggle};

/// The visible title of an operator.
///
/// The symbolic comparisons read the same in every language; the spelled-out
/// operators come from the catalog.
pub fn operator_title(language: Language, operator: FilterOperator) -> &'static str {
    match operator {
        FilterOperator::Equals => "=",
        FilterOperator::NotEquals => "≠",
        FilterOperator::Contains => tr(language, "Contains"),
        FilterOperator::NotContains => tr(language, "Not contains"),
        FilterOperator::BeginsWith => tr(language, "Begins with"),
        FilterOperator::EndsWith => tr(language, "Ends with"),
        FilterOperator::GreaterThan => ">",
        FilterOperator::LessThan => "<",
        FilterOperator::GreaterOrEqual => "≥",
        FilterOperator::LessOrEqual => "≤",
        FilterOperator::IsEmpty => tr(language, "Is empty"),
        FilterOperator::IsNotEmpty => tr(language, "Is not empty"),
        FilterOperator::MatchesRegex => tr(language, "Matches regex"),
    }
}

/// One filter-bar action chosen in this frame.
///
/// The bar edits the draft directly; the caller performs the action so it can
/// commit an open cell editor before rows change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterAction {
    /// Apply every enabled draft criterion.
    ApplyAll,
    /// Apply one draft criterion alone.
    Apply(CriterionId),
    /// Remove the applied filter and reset the draft.
    Clear,
    /// Insert an empty criterion after this one.
    Add(CriterionId),
    /// Remove a criterion, keeping one row.
    Remove(CriterionId),
    /// Hide the bar without discarding the draft or the filter.
    Hide,
}

impl FilterAction {
    /// Whether the action can change which rows are visible.
    pub fn changes_rows(self) -> bool {
        matches!(self, Self::ApplyAll | Self::Apply(_) | Self::Clear)
    }
}

/// Draws the filter bar for `document`; returns the action the user chose.
///
/// `keys_enabled` is false while a modal owns the keyboard; the bar still
/// draws, but its shortcuts are left for the alert.
///
/// The draft criteria are edited in place. Rows are re-evaluated only by the
/// caller's Apply actions, so typing never hides the row being edited.
pub fn show(
    ui: &mut Ui,
    document: &mut Document,
    language: Language,
    keys_enabled: bool,
) -> Option<FilterAction> {
    let columns = column_options(document);
    let (first_column, _) = columns.first()?;
    document
        .row_filter_mut()
        .fill_missing_columns(*first_column);

    let theme = current(ui.ctx());
    let mut criteria = document.row_filter().criteria.clone();
    let mut action = if keys_enabled {
        keyboard_action(ui, &criteria)
    } else {
        None
    };
    ui.spacing_mut().item_spacing.x = 6.0;

    for criterion in &mut criteria {
        ui.horizontal(|ui| {
            let mut enabled = criterion.is_enabled;
            let response = toggle(ui, "", &mut enabled)
                .accessible_name(tr(language, "Enable Criterion"))
                .on_hover_text(tr(language, "Enable Criterion"));
            if response.changed() {
                criterion.is_enabled = enabled;
            }

            let names: Vec<&str> = columns.iter().map(|(_, name)| name.as_str()).collect();
            let selected = criterion
                .column_id
                .and_then(|id| columns.iter().position(|(column, _)| *column == id))
                .unwrap_or(0);
            if let Some(index) = Picker::new(
                ("filter-column", criterion.id),
                tr(language, "Column"),
                names[selected],
                &names,
            )
            .width(130.0)
            .show(ui)
            {
                criterion.column_id = Some(columns[index].0);
            }

            let operators: Vec<&str> = FilterOperator::ALL
                .iter()
                .map(|operator| operator_title(language, *operator))
                .collect();
            let selected = FilterOperator::ALL
                .iter()
                .position(|operator| *operator == criterion.op)
                .unwrap_or(0);
            if let Some(index) = Picker::new(
                ("filter-operator", criterion.id),
                tr(language, "Operator"),
                operators[selected],
                &operators,
            )
            .width(150.0)
            .show(ui)
            {
                criterion.op = FilterOperator::ALL[index];
            }

            let needs_value = criterion.op.needs_value();
            let field = ui
                .add_enabled_ui(needs_value, |ui| {
                    text_field_sized(
                        ui,
                        tr(language, "Value"),
                        &mut criterion.value,
                        tr(language, "Value"),
                        160.0,
                    )
                })
                .inner;
            if !criterion.is_valid() {
                ui.painter().rect_stroke(
                    field.rect.expand(1.0),
                    CornerRadius::same(theme.metrics.small_corner_radius),
                    Stroke::new(1.5, theme.colors.error),
                    StrokeKind::Outside,
                );
                ui.label(
                    RichText::new(tr(language, "Invalid regular expression"))
                        .size(theme.typography.caption)
                        .color(theme.colors.error),
                );
            }
            let submitted = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
            if submitted
                || PushButton::new(tr(language, "Apply"), ButtonKind::Bordered)
                    .show(ui)
                    .clicked()
            {
                action = Some(FilterAction::Apply(criterion.id));
            }
            if PushButton::new("\u{2212}", ButtonKind::Plain)
                .accessible_name(tr(language, "Remove Criterion"))
                .show(ui)
                .clicked()
            {
                action = Some(FilterAction::Remove(criterion.id));
            }
            if PushButton::new("+", ButtonKind::Plain)
                .accessible_name(tr(language, "Add Criterion"))
                .show(ui)
                .clicked()
            {
                action = Some(FilterAction::Add(criterion.id));
            }
        });
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if PushButton::new(tr(language, "Hide"), ButtonKind::Plain)
            .show(ui)
            .clicked()
        {
            action = Some(FilterAction::Hide);
        }
        let hint_width = (ui.available_width() - 170.0).max(0.0);
        if hint_width > 0.0 {
            let hint = shortcut_hint(language);
            ui.allocate_ui_with_layout(
                Vec2::new(hint_width, theme.metrics.control_height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(hint)
                                .size(theme.typography.caption)
                                .color(theme.colors.secondary_label),
                        )
                        .truncate(),
                    );
                },
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if PushButton::new(tr(language, "Apply All"), ButtonKind::Bordered)
                .show(ui)
                .clicked()
            {
                action = Some(FilterAction::ApplyAll);
            }
            if PushButton::new(tr(language, "Clear"), ButtonKind::Bordered)
                .show(ui)
                .clicked()
            {
                action = Some(FilterAction::Clear);
            }
        });
    });

    document.row_filter_mut().criteria = criteria;
    action
}

/// The columns of `document`, named as the grid headers are.
fn column_options(document: &Document) -> Vec<(ColumnId, String)> {
    let Some(source) = document.row_source() else {
        return Vec::new();
    };
    let presenter = GridPresenter::new(source, None);
    source
        .column_ids()
        .into_iter()
        .enumerate()
        .map(|(index, id)| (id, presenter.header(index)))
        .collect()
}

/// The footer hint with this platform's modifier names.
fn shortcut_hint(language: Language) -> String {
    let text = tr(
        language,
        "Show ⌘F   Insert ⌘I   Remove ⌘⇧⌫   Apply All ⌘↩   Exit ⎋",
    );
    if cfg!(target_os = "macos") {
        return text.to_owned();
    }
    text.replace('⌘', "Ctrl+")
        .replace('⇧', "Shift+")
        .replace('⌫', "Backspace")
        .replace('↩', "Enter")
        .replace('⎋', "Esc")
}

/// The keyboard shortcuts the footer advertises.
///
/// They act on the last criterion, or on the bar itself, and never fire while
/// a text field has focus.
fn keyboard_action(ui: &Ui, criteria: &[tabtool_core::FilterCriterion]) -> Option<FilterAction> {
    let target = criteria.last()?.id;
    let text_focused = ui.ctx().text_edit_focused();
    ui.input_mut(|input| {
        if input.consume_key(Modifiers::COMMAND, Key::I) {
            Some(FilterAction::Add(target))
        } else if input.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Backspace)
            || input.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::Delete)
        {
            Some(FilterAction::Remove(target))
        } else if input.consume_key(Modifiers::COMMAND, Key::Enter) {
            Some(FilterAction::ApplyAll)
        } else if !text_focused && input.consume_key(Modifiers::NONE, Key::Escape) {
            Some(FilterAction::Hide)
        } else {
            None
        }
    })
}

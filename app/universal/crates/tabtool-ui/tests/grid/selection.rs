//! Tap resolution and selection rules.

use egui::Modifiers;
use tabtool_document::GridSelection as DocumentSelection;
use tabtool_ui::grid::{
    GridAnchor, GridElement, GridFocus, GridOptions, GridSelection, GridState, GridTapOutcome,
    resolve_tap,
};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

#[test]
fn first_tap_selects() {
    let cell = GridFocus::new(2, 3);

    assert_eq!(
        resolve_tap(GridElement::Cell, None, Some(cell)),
        GridTapOutcome::SelectCell
    );
}

#[test]
fn second_tap_on_the_focused_cell_edits() {
    let cell = GridFocus::new(2, 3);

    assert_eq!(
        resolve_tap(GridElement::Cell, Some(cell), Some(cell)),
        GridTapOutcome::EditCell
    );
}

#[test]
fn tap_on_another_cell_selects_instead_of_editing() {
    let cell = GridFocus::new(2, 3);
    let other = GridFocus::new(2, 4);

    assert_eq!(
        resolve_tap(GridElement::Cell, Some(cell), Some(other)),
        GridTapOutcome::SelectCell
    );
}

#[test]
fn header_and_row_number_select_their_axis() {
    let cell = GridFocus::new(2, 3);

    assert_eq!(
        resolve_tap(GridElement::ColumnHeader, Some(cell), None),
        GridTapOutcome::SelectColumn
    );
    assert_eq!(
        resolve_tap(GridElement::RowNumber, Some(cell), None),
        GridTapOutcome::SelectRowNumber
    );
}

#[test]
fn corner_and_empty_hits_are_ignored() {
    let cell = GridFocus::new(2, 3);

    assert_eq!(
        resolve_tap(GridElement::Corner, Some(cell), None),
        GridTapOutcome::Ignore
    );
    assert_eq!(
        resolve_tap(GridElement::Cell, None, None),
        GridTapOutcome::Ignore
    );
}

#[test]
fn a_plain_cell_click_selects_the_row_and_clears_columns() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();

    assert_eq!(fixture.selected_rows(), [0].into_iter().collect());
    assert!(fixture.selected_columns().is_empty());
}

#[test]
fn a_header_click_selects_the_column_and_clears_rows() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.get_by_label("Amount").click();
    fixture.run();

    assert!(fixture.selected_rows().is_empty());
    assert_eq!(fixture.selected_columns(), [1].into_iter().collect());
}

#[test]
fn command_click_toggles_columns() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Name").click();
    fixture.run();
    fixture
        .get_by_label("Note")
        .click_modifiers(Modifiers::COMMAND);
    fixture.run();

    assert_eq!(fixture.selected_columns(), [0, 2].into_iter().collect());

    fixture
        .get_by_label("Name")
        .click_modifiers(Modifiers::COMMAND);
    fixture.run();
    assert_eq!(fixture.selected_columns(), [2].into_iter().collect());
}

#[test]
fn shift_click_extends_the_column_range() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Name").click();
    fixture.run();
    fixture
        .get_by_label("Note")
        .click_modifiers(Modifiers::SHIFT);
    fixture.run();

    assert_eq!(fixture.selected_columns(), [0, 1, 2].into_iter().collect());
}

#[test]
fn shift_click_extends_the_row_range() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture
        .get_by_label("Carol")
        .click_modifiers(Modifiers::SHIFT);
    fixture.run();

    assert_eq!(fixture.selected_rows(), [0, 1, 2].into_iter().collect());
}

#[test]
fn command_click_toggles_rows() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture
        .get_by_label("Carol")
        .click_modifiers(Modifiers::COMMAND);
    fixture.run();

    assert_eq!(fixture.selected_rows(), [0, 2].into_iter().collect());

    fixture
        .get_by_label("Alice")
        .click_modifiers(Modifiers::COMMAND);
    fixture.run();
    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());
}

#[test]
fn selection_survives_a_row_count_reload() {
    let table = sample_table();
    let options = GridOptions::default();
    let mut state = GridState::default();
    state.sync(&table, &options);
    state.selection.rows.insert(0);
    state.focus = Some(GridFocus::new(0, 0));

    let larger = CountingSource::new(10, 3);
    state.sync(&larger, &options);

    assert_eq!(state.selection.rows, [0].into_iter().collect());
    assert_eq!(state.focus, Some(GridFocus::new(0, 0)));
}

#[test]
fn a_selection_is_either_rows_or_columns() {
    let mut selection = GridSelection::default();
    let mut anchor = None;
    selection.select_column(1, false, false, &mut anchor);
    selection.select_row(2, false, false, &mut anchor);

    assert_eq!(selection.rows, [2].into_iter().collect());
    assert!(selection.columns.is_empty());

    selection.select_column(0, false, false, &mut anchor);
    assert!(selection.rows.is_empty());
    assert_eq!(selection.columns, [0].into_iter().collect());
}

#[test]
fn arrow_keys_move_the_selected_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.key_press(egui::Key::ArrowDown);
    fixture.run();

    assert_eq!(fixture.selected_rows(), [1].into_iter().collect());
    assert_eq!(
        fixture
            .state
            .borrow()
            .focus
            .map(|focus| (focus.row, focus.column)),
        Some((1, 0))
    );
}

#[test]
fn shift_arrow_extends_the_selected_rows() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.key_press_modifiers(Modifiers::SHIFT, egui::Key::ArrowDown);
    fixture.run();

    assert_eq!(fixture.selected_rows(), [0, 1].into_iter().collect());
}

#[test]
fn shift_after_an_axis_change_uses_the_new_axis_anchor() {
    let mut selection = GridSelection::default();
    let mut anchor = None;
    selection.select_column(2, false, false, &mut anchor);
    assert_eq!(anchor, Some(GridAnchor::Column(2)));

    selection.select_row(3, true, false, &mut anchor);
    assert_eq!(
        selection.rows,
        [3].into_iter().collect(),
        "a column anchor must not define a row range"
    );
    assert!(selection.columns.is_empty());

    selection.select_row(0, false, false, &mut anchor);
    selection.select_column(2, true, false, &mut anchor);
    assert_eq!(
        selection.columns,
        [2].into_iter().collect(),
        "a row anchor must not define a column range"
    );
}

#[test]
fn document_selection_roundtrips_losslessly() {
    let mut document = DocumentSelection::default();
    document.rows.insert(1);
    document.rows.insert(4);

    let mut state = GridState::default();
    state.apply_document_selection(&document);
    assert_eq!(state.selection.rows, document.rows);
    assert!(state.selection.columns.is_empty());
    assert_eq!(state.document_selection(), document);

    let mut columns = DocumentSelection::default();
    columns.columns.insert(2);
    state.apply_document_selection(&columns);
    assert_eq!(state.document_selection(), columns);
    assert_eq!(DocumentSelection::from(state.selection.clone()), columns);
}

#[test]
fn shift_click_after_a_column_click_selects_only_the_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Note").click();
    fixture.run();
    fixture
        .get_by_label("Dave")
        .click_modifiers(Modifiers::SHIFT);
    fixture.run();

    assert_eq!(fixture.selected_rows(), [3].into_iter().collect());
    assert!(fixture.selected_columns().is_empty());
}

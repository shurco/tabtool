//! Chrome commands must commit the open cell editor before they change rows,
//! columns or the header, so typed text lands in the cell it came from.

use tabtool_core::CsvConfiguration;
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{Fixture, MemoryStorage, loaded};

fn fixture() -> Fixture {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, CsvConfiguration::default()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();
    fixture
}

fn rows(rows: impl IntoIterator<Item = usize>) -> tabtool_ui::grid::GridSelection {
    tabtool_ui::grid::GridSelection {
        rows: rows.into_iter().collect(),
        columns: Default::default(),
    }
}

fn columns(columns: impl IntoIterator<Item = usize>) -> tabtool_ui::grid::GridSelection {
    tabtool_ui::grid::GridSelection {
        rows: Default::default(),
        columns: columns.into_iter().collect(),
    }
}

fn cell(fixture: &Fixture, row: usize, column: usize) -> String {
    fixture.with_document(|document| {
        document
            .table()
            .expect("the fixture has a table")
            .cell(row, column)
    })
}

#[test]
fn add_row_above_commits_the_editor_into_the_original_row() {
    let mut fixture = fixture();
    fixture.set_selection(rows([0]));
    fixture.edit_cell(0, 0, "typed");
    fixture.run();

    fixture.get_by_label("Add Row Above").click();
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.table().expect("table").row_count()),
        3
    );
    assert_eq!(cell(&fixture, 0, 0), "", "the inserted row is blank");
    assert_eq!(
        cell(&fixture, 1, 0),
        "typed",
        "the text must move with its original row"
    );
}

#[test]
fn add_column_left_commits_the_editor_into_the_original_column() {
    let mut fixture = fixture();
    fixture.set_selection(columns([0]));
    fixture.edit_cell(0, 0, "typed");
    fixture.run();

    fixture.get_by_label("Add Column Left").click();
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.table().expect("table").columns.len()),
        3
    );
    assert_eq!(cell(&fixture, 0, 0), "", "the inserted column is blank");
    assert_eq!(
        cell(&fixture, 0, 1),
        "typed",
        "the text must move with its original column"
    );
}

#[test]
fn delete_commit_does_not_write_the_editor_into_the_next_row() {
    let mut fixture = fixture();
    fixture.set_selection(rows([0]));
    fixture.edit_cell(0, 0, "typed");
    fixture.run();

    fixture.get_by_label("Delete Selection").click();
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.table().expect("table").row_count()),
        1
    );
    assert_eq!(
        cell(&fixture, 0, 0),
        "1",
        "the deleted row's text must not overwrite the next row"
    );
}

#[test]
fn a_row_context_menu_commits_the_editor_into_the_original_row() {
    let storage = MemoryStorage::new(b"a,b\nx,y\n");
    let mut fixture = Fixture::new(
        loaded(&storage, CsvConfiguration::default()),
        MemoryStorage::new(b""),
        Default::default(),
    );
    fixture.run();
    fixture.set_selection(rows([0]));
    fixture.edit_cell(0, 0, "typed");
    fixture.run();

    fixture.get_by_label("1").click_secondary();
    fixture.run();
    fixture.run();
    fixture
        .get_all_by_label("Add Row Above")
        .last()
        .expect("the row menu has Add Row Above")
        .click();
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.table().expect("table").row_count()),
        3
    );
    assert_eq!(cell(&fixture, 0, 0), "", "the inserted row is blank");
    assert_eq!(
        cell(&fixture, 1, 0),
        "typed",
        "the context-menu insert must not take the text meant for row 0"
    );
}

#[test]
fn header_toggle_commits_the_editor_before_the_rows_shift() {
    let mut fixture = fixture();
    fixture.edit_cell(0, 0, "typed");
    fixture.run();

    fixture
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Header")
        .click();
    fixture.run();

    let headers =
        fixture.with_document(|document| document.table().and_then(|table| table.headers()));
    assert_eq!(
        headers.as_deref(),
        Some(["typed".to_owned(), "b".to_owned()].as_slice()),
        "the committed row becomes the header"
    );
    assert_eq!(
        cell(&fixture, 0, 0),
        "1",
        "the first data row is the old second row"
    );
}

#[test]
fn a_modal_blocks_grid_keys() {
    let mut fixture = fixture();
    fixture.run();
    fixture.set_selection(rows([0]));
    fixture.set_input_blocked(true);

    fixture.key_press(egui::Key::Delete);
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.table().expect("table").row_count()),
        2,
        "Delete must not reach a grid under a modal"
    );
}

#[test]
fn chrome_actions_do_not_lose_the_editor_when_it_is_unchanged() {
    let mut fixture = fixture();
    fixture.set_selection(rows([0]));
    fixture.edit_cell(0, 0, "a");
    fixture.run();

    fixture.get_by_label("Add Row Below").click();
    fixture.run();

    assert_eq!(cell(&fixture, 0, 0), "a");
    assert_eq!(cell(&fixture, 1, 0), "");
}

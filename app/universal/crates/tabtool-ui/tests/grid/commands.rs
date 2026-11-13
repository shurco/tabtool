//! Grid command dispatch to the document and keyboard navigation.

use std::collections::BTreeSet;

use egui::{Event, Key, Modifiers};
use tabtool_core::{
    CsvConfiguration, FilterCriterion, FilterOperator, Locale, RowSource, SortDirection,
};
use tabtool_document::{
    Document, DocumentError, DocumentNotice, GridSelection as DocumentSelection, UndoAction,
};
use tabtool_ui::grid::{Clipboard, GridAction, GridOptions, GridState, apply_action};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

fn locale() -> Locale {
    "en-US".parse().expect("en-US is a valid locale")
}

/// A clipboard that keeps its text in memory.
#[derive(Default)]
struct FakeClipboard {
    text: Option<String>,
}

impl FakeClipboard {
    fn with_text(text: &str) -> Self {
        Self {
            text: Some(text.to_owned()),
        }
    }
}

impl Clipboard for FakeClipboard {
    fn copy(&mut self, text: &str) {
        self.text = Some(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        self.text.clone()
    }
}

fn document(csv: &str) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn header_document(csv: &str) -> Document {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn run(
    document: &mut Document,
    action: GridAction,
    clipboard: &mut FakeClipboard,
) -> Result<(), DocumentError> {
    apply_action(document, action, &locale(), ".", clipboard)
}

fn select_rows(document: &mut Document, rows: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = DocumentSelection {
        rows: rows.into_iter().collect(),
        columns: BTreeSet::new(),
    };
}

fn select_columns(document: &mut Document, columns: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = DocumentSelection {
        rows: BTreeSet::new(),
        columns: columns.into_iter().collect(),
    };
}

fn cells(document: &Document) -> Vec<Vec<String>> {
    let table = document.table().expect("the fixture has a table");
    (0..table.columns.len())
        .map(|column| table.columns[column].cells.clone())
        .collect()
}

/// A document whose first column equals `a` for rows 0 and 2.
fn filtered_document() -> Document {
    let mut document = header_document("h,value\na,1\nb,2\na,3\n");
    let column_id = document.table().expect("table").column_ids()[0];
    document.row_filter_mut().criteria = vec![FilterCriterion {
        id: 1,
        is_enabled: true,
        column_id: Some(column_id),
        op: FilterOperator::Equals,
        value: "a".to_owned(),
    }];
    document.apply_all_filters();
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    document
}

/// A large document with `rows` data rows and three columns.
fn large_document(rows: usize) -> (tempfile::TempDir, Document) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    let mut csv = String::from("h1,h2,h3\n");
    for row in 0..rows {
        csv.push_str(&format!("r{row},a{row},b{row}\n"));
    }
    std::fs::write(&path, csv).expect("write the fixture");
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(&path, None, false, 0)
        .expect("open the fixture large");
    document.large_state().expect("large state").indexer.wait();
    document.poll_large_indexing();
    (directory, document)
}

fn grid(source: &dyn tabtool_core::RowSource) -> GridFixture<'_> {
    GridFixture::new(
        source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    )
}

// ---------------------------------------------------------------------------
// Document commands
// ---------------------------------------------------------------------------

#[test]
fn commit_cell_writes_one_undoable_cell() {
    let mut document = document("a,b\nc,d\n");
    let mut clipboard = FakeClipboard::default();

    run(
        &mut document,
        GridAction::CommitCell {
            row: 1,
            column: 0,
            text: "x".to_owned(),
        },
        &mut clipboard,
    )
    .expect("commit the cell");

    assert_eq!(document.table().expect("table").cell(1, 0), "x");
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(1, 0), "c");
}

#[test]
fn add_row_uses_the_selection_and_records_add_row() {
    let mut document = document("0\n1\n");
    select_rows(&mut document, [1]);
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::AddRowBelow, &mut clipboard).expect("add a row");

    assert_eq!(document.table().expect("table").row_count(), 3);
    assert_eq!(document.undo_action(), Some(UndoAction::AddRow));
    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
}

#[test]
fn add_column_left_and_right_insert_around_the_selection() {
    let mut document = header_document("h1,h2\na,b\n");
    select_columns(&mut document, [1]);
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::AddColumnRight, &mut clipboard).expect("add a column");
    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 3);
    assert_eq!(
        table.headers(),
        Some(vec!["h1".to_owned(), "h2".to_owned(), String::new()])
    );
    assert_eq!(table.cell(0, 1), "b");
    assert_eq!(table.cell(0, 2), "");

    select_columns(&mut document, [0]);
    run(&mut document, GridAction::AddColumnLeft, &mut clipboard).expect("add a column");
    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 4);
    assert_eq!(
        table.headers(),
        Some(vec![
            String::new(),
            "h1".to_owned(),
            "h2".to_owned(),
            String::new()
        ])
    );
    assert_eq!(table.cell(0, 0), "");
    assert_eq!(table.cell(0, 1), "a");
}

#[test]
fn delete_selection_prefers_rows_over_columns() {
    let mut document = document("a,b\nc,d\n");
    *document.selection_mut() = DocumentSelection {
        rows: [0].into_iter().collect(),
        columns: [1].into_iter().collect(),
    };
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::DeleteSelection, &mut clipboard).expect("delete");

    assert_eq!(document.table().expect("table").row_count(), 1);
    assert_eq!(document.table().expect("table").columns.len(), 2);
    assert_eq!(document.undo_action(), Some(UndoAction::DeleteRows));
    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
}

#[test]
fn sorting_the_header_sets_the_arrow_and_remove_sort_restores_the_rows() {
    let mut document = header_document("h,1\nb,x\na,y\n");
    let mut clipboard = FakeClipboard::default();

    run(
        &mut document,
        GridAction::Sort {
            column: 0,
            direction: SortDirection::Ascending,
        },
        &mut clipboard,
    )
    .expect("sort");
    assert_eq!(cells(&document)[0], ["a", "b"]);
    assert_eq!(document.undo_action(), Some(UndoAction::SortRows));
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));

    run(&mut document, GridAction::RemoveSort, &mut clipboard).expect("remove the sort");
    assert_eq!(cells(&document)[0], ["b", "a"]);
    assert_eq!(document.undo_action(), Some(UndoAction::RemoveSort));
    assert_eq!(document.active_sort(), None);
}

#[test]
fn rename_column_applies_once_and_accepts_an_empty_name() {
    let mut document = header_document("a,b\n1,2\n");
    let mut clipboard = FakeClipboard::default();

    run(
        &mut document,
        GridAction::RenameColumn {
            column: 1,
            name: "x".to_owned(),
        },
        &mut clipboard,
    )
    .expect("rename");
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec!["a".to_owned(), "x".to_owned()])
    );
    assert_eq!(document.undo_action(), Some(UndoAction::RenameColumn));

    run(
        &mut document,
        GridAction::RenameColumn {
            column: 0,
            name: String::new(),
        },
        &mut clipboard,
    )
    .expect("rename to empty");
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec![String::new(), "x".to_owned()])
    );
}

#[test]
fn column_move_keeps_the_moved_column_selected_and_is_one_undo_step() {
    let mut document = header_document("a,b,c\n1,2,3\n");
    select_columns(&mut document, [0]);
    let mut clipboard = FakeClipboard::default();

    run(
        &mut document,
        GridAction::MoveColumn { from: 0, to: 3 },
        &mut clipboard,
    )
    .expect("move the column");

    let table = document.table().expect("table");
    assert_eq!(
        table.headers(),
        Some(vec!["b".to_owned(), "c".to_owned(), "a".to_owned()])
    );
    assert_eq!(cells(&document), [["2"], ["3"], ["1"]]);
    assert_eq!(document.selection().columns, [2].into_iter().collect());
    assert_eq!(document.undo_action(), Some(UndoAction::MoveColumn));
    assert!(document.undo());
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
    );
}

#[test]
fn row_move_selects_the_moved_rows_and_is_one_undo_step() {
    let mut document = document("a\nb\nc\n");
    let mut clipboard = FakeClipboard::default();

    run(
        &mut document,
        GridAction::MoveRows {
            from: [0, 2].into_iter().collect(),
            to: 3,
        },
        &mut clipboard,
    )
    .expect("move the rows");

    assert_eq!(cells(&document)[0], ["b", "a", "c"]);
    assert_eq!(document.selection().rows, [1, 2].into_iter().collect());
    assert_eq!(document.undo_action(), Some(UndoAction::MoveRows));
    assert!(document.undo());
    assert_eq!(cells(&document)[0], ["a", "b", "c"]);
}

// ---------------------------------------------------------------------------
// Copy and paste
// ---------------------------------------------------------------------------

#[test]
fn copy_writes_the_rows_as_tsv() {
    let mut document = document("a,b\nc,d\n");
    select_rows(&mut document, [0]);
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::Copy, &mut clipboard).expect("copy");

    assert_eq!(clipboard.text.as_deref(), Some("a\tb"));
}

#[test]
fn copy_without_a_selection_leaves_the_clipboard_alone() {
    let mut document = document("a,b\n");
    let mut clipboard = FakeClipboard::with_text("kept");

    run(&mut document, GridAction::Copy, &mut clipboard).expect("copy");

    assert_eq!(clipboard.text.as_deref(), Some("kept"));
}

#[test]
fn copy_of_a_filtered_column_uses_the_visible_rows() {
    let mut document = filtered_document();
    select_columns(&mut document, [1]);
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::Copy, &mut clipboard).expect("copy");

    assert_eq!(clipboard.text.as_deref(), Some("1\n3"));
}

#[test]
fn a_copy_over_the_large_file_limit_raises_the_notice_and_keeps_the_clipboard() {
    let (_directory, mut document) = large_document(340_000);
    select_rows(&mut document, 0..340_000);
    let mut clipboard = FakeClipboard::with_text("kept");

    run(&mut document, GridAction::Copy, &mut clipboard).expect("copy is refused, not failed");

    assert_eq!(
        document.take_notice(),
        Some(DocumentNotice::CopyLimitExceeded)
    );
    assert_eq!(clipboard.text.as_deref(), Some("kept"));
}

#[test]
fn paste_reads_the_clipboard_and_selects_the_inserted_rows() {
    let mut document = document("a,b\n");
    select_rows(&mut document, [0]);
    let mut clipboard = FakeClipboard::with_text("x\ty");

    run(&mut document, GridAction::Paste, &mut clipboard).expect("paste");

    assert_eq!(document.table().expect("table").row_count(), 2);
    assert_eq!(document.table().expect("table").cell(1, 0), "x");
    assert_eq!(document.table().expect("table").cell(1, 1), "y");
    assert_eq!(document.selection().rows, [1].into_iter().collect());
    assert_eq!(document.undo_action(), Some(UndoAction::PasteData));
}

#[test]
fn paste_keeps_an_empty_string_as_one_cell() {
    let mut document = document("a,b\n");
    let mut clipboard = FakeClipboard::with_text("");

    run(&mut document, GridAction::Paste, &mut clipboard).expect("paste");

    assert_eq!(document.table().expect("table").row_count(), 2);
    assert_eq!(document.selection().rows, [1].into_iter().collect());
}

#[test]
fn paste_ignores_an_empty_clipboard() {
    let mut document = document("a,b\n");
    let mut clipboard = FakeClipboard::default();
    let revision = document.revision();

    run(&mut document, GridAction::Paste, &mut clipboard).expect("paste");

    assert_eq!(document.revision(), revision);
    assert_eq!(document.table().expect("table").row_count(), 1);
}

#[test]
fn paste_is_refused_while_filtering() {
    let mut document = filtered_document();
    select_rows(&mut document, [1]);
    let mut clipboard = FakeClipboard::with_text("x");
    let revision = document.revision();

    run(&mut document, GridAction::Paste, &mut clipboard).expect("paste");

    assert_eq!(document.revision(), revision);
}

#[test]
fn select_all_selects_every_row() {
    let mut document = document("a\nb\n");
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::SelectAll, &mut clipboard).expect("select all");

    assert_eq!(document.selection().rows, [0, 1].into_iter().collect());
    assert!(document.selection().columns.is_empty());
}

#[test]
fn select_all_selects_only_the_visible_rows() {
    let mut document = filtered_document();
    let mut clipboard = FakeClipboard::default();

    run(&mut document, GridAction::SelectAll, &mut clipboard).expect("select all");

    assert_eq!(document.selection().rows, [0, 2].into_iter().collect());
}

#[test]
fn commands_need_loaded_content() {
    let mut document = Document::new(CsvConfiguration::default());
    let mut clipboard = FakeClipboard::default();

    assert_eq!(
        run(&mut document, GridAction::Copy, &mut clipboard),
        Err(DocumentError::NotReady)
    );
    assert_eq!(
        run(
            &mut document,
            GridAction::RenameColumn {
                column: 0,
                name: "x".to_owned(),
            },
            &mut clipboard,
        ),
        Err(DocumentError::NotReady)
    );
}

// ---------------------------------------------------------------------------
// Keyboard navigation
// ---------------------------------------------------------------------------

fn focus(fixture: &GridFixture<'_>) -> Option<(usize, usize)> {
    fixture
        .state
        .borrow()
        .focus
        .map(|focus| (focus.row, focus.column))
}

#[test]
fn tab_moves_right_and_wraps_to_the_next_row() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();

    fixture.key_press(Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 1)));
    fixture.key_press(Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 2)));
    fixture.key_press(Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), Some((1, 0)));
    assert_eq!(fixture.selected_rows(), [1].into_iter().collect());
}

#[test]
fn shift_tab_moves_left_and_wraps_to_the_previous_row() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.key_press(Key::ArrowDown);
    fixture.run();

    fixture.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 2)));
}

#[test]
fn home_and_end_move_to_the_row_edges() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();

    fixture.key_press(Key::End);
    fixture.run();
    assert_eq!(focus(&fixture), Some((1, 2)));

    fixture.key_press(Key::Home);
    fixture.run();
    assert_eq!(focus(&fixture), Some((1, 0)));
}

#[test]
fn page_keys_move_by_the_visible_row_count() {
    let source = CountingSource::new(100, 3);
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("r0c0").click();
    fixture.run();

    fixture.key_press(Key::PageDown);
    fixture.run();
    assert_eq!(focus(&fixture), Some((16, 0)));
    assert_eq!(fixture.selected_rows(), [16].into_iter().collect());

    fixture.key_press(Key::PageUp);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 0)));
}

#[test]
fn command_arrows_move_to_the_grid_edges() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowDown);
    fixture.run();
    assert_eq!(focus(&fixture), Some((3, 0)));
    assert_eq!(fixture.selected_rows(), [3].into_iter().collect());

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowRight);
    fixture.run();
    assert_eq!(focus(&fixture), Some((3, 2)));

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowUp);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 2)));

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowLeft);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 0)));
}

#[test]
fn command_arrows_keep_the_focus_in_a_wide_grid() {
    let source = CountingSource::new(5, 40);
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("r0c0").click();
    fixture.run();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowRight);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 39)));

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowLeft);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 0)));
}

#[test]
fn delete_emits_delete_selection() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.take_actions();

    fixture.key_press(Key::Delete);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![GridAction::DeleteSelection]);
}

#[test]
fn command_a_selects_every_row_and_emits_select_all() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.take_actions();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::A);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![GridAction::SelectAll]);
    assert_eq!(fixture.selected_rows(), [0, 1, 2, 3].into_iter().collect());
}

#[test]
fn copy_and_paste_shortcuts_emit_clipboard_actions() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.take_actions();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::C);
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![GridAction::Copy]);

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::V);
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![GridAction::Paste]);
}

#[test]
fn clipboard_events_emit_clipboard_actions() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();
    fixture.take_actions();

    fixture.event(Event::Copy);
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![GridAction::Copy]);

    fixture.event(Event::Paste("x".to_owned()));
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![GridAction::Paste]);
}

#[test]
fn no_grid_command_fires_while_typing_in_a_cell() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    assert!(fixture.state.borrow().editing.is_some());
    fixture.take_actions();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::A);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "Command+A must stay with the text field"
    );
    fixture.key_press(Key::Delete);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "Delete must stay with the text field"
    );
}

#[test]
fn a_read_only_grid_still_navigates_but_does_not_edit() {
    let source = sample_table();
    let options = GridOptions {
        editable: false,
        ..GridOptions::default()
    };
    let mut fixture = GridFixture::new(&source, GridState::default(), options, [640.0, 400.0]);
    fixture.run();
    fixture.get_by_label("Alice").click();
    fixture.run();

    fixture.key_press(Key::ArrowRight);
    fixture.run();
    assert_eq!(focus(&fixture), Some((0, 1)));

    fixture.key_press(Key::Enter);
    fixture.run();
    assert!(fixture.state.borrow().editing.is_none());
}

#[test]
fn the_grid_does_not_react_without_a_focused_cell() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();

    fixture.key_press(Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), None);
    assert!(fixture.take_actions().is_empty());
}

#[test]
fn scroll_to_requests_keep_the_navigated_cell_visible() {
    let source = CountingSource::new(100, 3);
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("r0c0").click();
    fixture.run();

    fixture.key_press_modifiers(Modifiers::COMMAND, Key::ArrowDown);
    fixture.run();
    assert_eq!(focus(&fixture), Some((99, 0)));
    assert!(
        fixture.state.borrow().scroll.y > 0.0,
        "moving to the last row must scroll it into view"
    );
}

#[test]
fn a_tab_key_reaches_a_cell_focused_by_a_row_number_click() {
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("3").click();
    fixture.run();
    assert_eq!(focus(&fixture), Some((2, 0)));

    fixture.key_press(Key::Tab);
    fixture.run();
    assert_eq!(focus(&fixture), Some((2, 1)));
}

#[test]
fn the_grid_returns_a_commit_instead_of_mutating_the_source() {
    // The grid keeps returning actions; the dispatcher is what mutates the
    // document.
    let source = sample_table();
    let mut fixture = grid(&source);
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    fixture.key_press(Key::Enter);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::CommitCell {
            row: 1,
            column: 0,
            text: "Bob".to_owned(),
        }]
    );
}

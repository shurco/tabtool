//! The touch editor: native text input, accessory moves, action bar routing,
//! read-only large mode and the large status/banner stack.

use egui::accesskit::Role;
use tabtool_ui::document::LargeUiAction;
use tabtool_ui::grid::GridFocus;
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{MobileEditTarget, TouchTextInputOutcome};

use crate::support::{self, EditorFixture};

fn open_cell(fixture: &mut EditorFixture, text: &str) {
    fixture.click_cell(text);
    fixture.click_cell(text);
}

#[test]
fn the_editor_composes_the_toolbar_grid_and_status() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();

    assert!(
        fixture.query_by_label_contains("Budget.csv").is_some(),
        "the toolbar shows the title"
    );
    assert!(fixture.has_label("Documents"), "the toolbar goes back");
    assert!(fixture.has_label("Alice"), "the grid paints the rows");
    assert!(
        fixture.query_by_label_contains("rows ×").is_some(),
        "the status line reports the table size"
    );
}

#[test]
fn the_overflow_offers_share_only_for_a_file_backed_document() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    fixture.click("More");
    assert!(
        !fixture.has_label("Share"),
        "an untitled document has nothing to share"
    );

    let mut document = support::small_document();
    document
        .set_file_path(Some(std::path::Path::new("/tmp/Budget.csv")))
        .expect("the fixture is file-backed");
    let mut fixture = EditorFixture::phone(document);
    fixture.run();
    fixture.click("More");
    assert!(
        fixture.has_label("Share"),
        "a file-backed document offers Share"
    );
}

#[test]
fn a_fast_second_tap_is_the_edit_gesture_and_never_zooms() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    let zoom = fixture.ctx.zoom_factor();

    fixture.click_cell("Alice");
    assert!(
        fixture.edit_target().is_none(),
        "the first tap only selects the row"
    );
    assert!(
        fixture.input.request().is_none(),
        "the first tap must not open the native editor"
    );

    fixture.click_cell("Alice");
    let request = fixture
        .input
        .request()
        .expect("the second tap opens the native editor");
    assert_eq!(request.text, "Alice");
    assert!(!request.multiline, "a cell is a single line");
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 0 })
    );

    let cell = fixture.get_by_role_and_label(Role::Cell, "Alice").rect();
    assert!(
        (request.rect.min.x - cell.min.x).abs() < 1.0
            && (request.rect.min.y - cell.min.y).abs() < 1.0
            && (request.rect.width() - cell.width()).abs() < 1.0,
        "the editor opens over the cell: request {:?}, cell {cell:?}",
        request.rect
    );
    assert_eq!(
        fixture.ctx.zoom_factor(),
        zoom,
        "the reserved double-tap gesture must not zoom the grid"
    );
}

#[test]
fn a_commit_writes_the_cell_exactly_once() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");

    fixture.input.set_draft("Alicia");
    fixture.click("Done");

    assert_eq!(fixture.cell(0, 0), "Alicia");
    assert_eq!(fixture.edit_target(), None, "Done closes the editor");
    assert_eq!(fixture.input.commits(), 1);

    fixture.run();
    fixture.run();
    assert_eq!(fixture.cell(0, 0), "Alicia");
    assert_eq!(
        fixture.input.commits(),
        1,
        "one session must produce one commit"
    );
}

#[test]
fn a_cancel_leaves_the_cell_unchanged() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");

    fixture.input.finish(TouchTextInputOutcome::Cancelled);
    fixture.run();

    assert_eq!(fixture.cell(0, 0), "Alice");
    assert_eq!(fixture.edit_target(), None);
    assert!(!fixture.document.borrow().can_undo());
}

#[test]
fn next_commits_and_moves_to_the_next_cell() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");

    fixture.input.set_draft("Alicia");
    fixture.click("Next Cell");

    assert_eq!(fixture.cell(0, 0), "Alicia");
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 1 })
    );
    assert_eq!(
        fixture.input.request().map(|request| request.text),
        Some("1.5".to_owned()),
        "the next cell opens with its own text"
    );
}

#[test]
fn next_wraps_to_the_next_row_and_previous_wraps_back() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");

    fixture.click("Next Cell");
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 1 })
    );

    fixture.click("Next Cell");
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 1, column: 0 }),
        "the last column wraps to the next row"
    );
    assert_eq!(
        fixture.input.request().map(|request| request.text),
        Some("Bob".to_owned())
    );

    fixture.click("Previous Cell");
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 1 }),
        "the first column wraps back to the previous row"
    );
}

#[test]
fn a_commit_never_replays_into_the_next_cell() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");

    fixture.input.set_draft("Alicia");
    fixture.click("Next Cell");
    fixture.run();
    fixture.run();

    assert_eq!(fixture.cell(0, 0), "Alicia");
    assert_eq!(
        fixture.cell(0, 1),
        "1.5",
        "the committed text must not leak into the cell opened afterwards"
    );
}

#[test]
fn a_header_tap_selects_the_column_and_rename_opens_the_native_editor() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();

    fixture.click_header("Name");
    assert_eq!(fixture.selection().columns, [0].into_iter().collect());
    assert!(fixture.has_label("Rename Column"));

    fixture.click("Rename Column");
    let request = fixture
        .input
        .request()
        .expect("Rename opens the native editor");
    assert_eq!(request.text, "Name");
    assert!(
        matches!(fixture.edit_target(), Some(MobileEditTarget::Column(_))),
        "rename targets the column identity"
    );
    assert!(
        !fixture.has_label("Next Cell") && !fixture.has_label("Previous Cell"),
        "a header rename has no cells to step through"
    );
    assert!(fixture.has_label("Done"));

    fixture.input.set_draft("Title");
    fixture.click("Done");

    let headers = fixture
        .document
        .borrow()
        .table()
        .and_then(|table| table.headers());
    assert_eq!(
        headers.as_deref(),
        Some(["Title".to_owned(), "Amount".to_owned()].as_slice())
    );
}

#[test]
fn a_cancelled_rename_keeps_the_header() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    fixture.click_header("Name");
    fixture.click("Rename Column");

    fixture.input.finish(TouchTextInputOutcome::Cancelled);
    fixture.run();

    let headers = fixture
        .document
        .borrow()
        .table()
        .and_then(|table| table.headers());
    assert_eq!(
        headers.as_deref(),
        Some(["Name".to_owned(), "Amount".to_owned()].as_slice())
    );
}

#[test]
fn a_toolbar_action_waits_for_the_open_edit_to_commit() {
    let mut document = support::small_document();
    document.set_cell(0, 0, "Base");
    let mut fixture = EditorFixture::phone(document);
    fixture.run();
    open_cell(&mut fixture, "Base");
    fixture.input.set_draft("Edited");

    fixture.get_by_label("Undo").click();
    fixture.run();

    let response = fixture.take_response();
    assert!(
        !response.toolbar.undo,
        "the editor applies document undo itself after the commit"
    );
    assert_eq!(
        fixture.input.commits(),
        1,
        "the open edit is committed exactly once before the undo"
    );
    assert_eq!(
        fixture.cell(0, 0),
        "Base",
        "the edit is committed and then undone, never undone first"
    );
}

#[test]
fn clearing_the_selection_drops_the_focus() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    fixture.click_cell("Bob");

    assert!(fixture.state.borrow().focus.is_some());
    fixture.document.borrow_mut().selection_mut().rows.clear();
    fixture.run();

    assert_eq!(fixture.state.borrow().focus, None);
    assert!(fixture.selection().is_empty());
}

#[test]
fn read_only_large_mode_refuses_the_native_editor() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let mut document = support::large_document(&directory);
    document.force_indexing_state_for_test(0.4);
    let mut fixture = EditorFixture::phone(document).keep(directory);
    fixture.run();

    fixture.click_cell("Alice");
    fixture.click_cell("Alice");

    assert_eq!(fixture.edit_target(), None);
    assert_eq!(
        fixture.input.request(),
        None,
        "an indexing file cannot open the cell editor"
    );
}

#[test]
fn the_edit_rejected_notice_is_shown_and_dismissed() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let mut document = support::large_document(&directory);
    document.force_indexing_state_for_test(0.4);
    document.poll_large_indexing();
    document.set_cell(0, 0, "X");
    let mut fixture = EditorFixture::phone(document).keep(directory);
    fixture.run();

    assert!(fixture.has_label("Edit Not Applied"));

    fixture.click("OK");
    assert!(!fixture.has_label("Edit Not Applied"));
}

#[test]
fn the_status_line_reports_save_progress_and_cancel() {
    let mut document = support::small_document();
    document.force_saving_state_for_test(0.5);
    let mut fixture = EditorFixture::phone(document);
    fixture.run();

    assert!(
        fixture.query_by_label_contains("Saving… 50%").is_some(),
        "the status line reports the streaming save"
    );
    fixture.click("Cancel");
    assert_eq!(
        fixture.take_response().large,
        Some(LargeUiAction::CancelSave)
    );
}

// Unix only: Windows refuses to rewrite a file while it is mapped.
#[cfg(unix)]
#[test]
fn the_stale_banner_offers_reload() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let mut document = support::large_document(&directory);
    let path = directory.path().join("large.csv");
    std::fs::write(&path, "Name,Amount\nAlice,1\nBob,2\nCarol,3\nDora,4\n")
        .expect("the file grows");
    document.refresh_large_staleness();
    let mut fixture = EditorFixture::phone(document).keep(directory);
    fixture.run();

    assert!(fixture.has_label("Reload"));
    let text = fixture
        .get_by_label("New rows were appended. Reload to see them.")
        .rect();
    let reload = fixture.get_by_label("Reload").rect();
    assert!(
        !text.intersects(reload),
        "the stale text {text:?} must not run under the Reload button {reload:?}"
    );
    fixture.click("Reload");
    assert_eq!(
        fixture.take_response().large,
        Some(LargeUiAction::ReloadStale)
    );
}

#[test]
fn a_grid_action_uses_the_touch_selection() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();

    fixture.click_cell("Bob");
    assert_eq!(fixture.selection().rows, [1].into_iter().collect());
    fixture.click("Add Row Below");

    let document = fixture.document.borrow();
    let table = document.table().expect("the fixture has a table");
    assert_eq!(table.row_count(), 4);
    assert_eq!(table.cell(2, 0), "", "the inserted row is blank");
    assert_eq!(table.cell(1, 0), "Bob", "the original row stays in place");
    assert_eq!(
        document.selection().rows,
        [2].into_iter().collect(),
        "the inserted row becomes the selection"
    );
}

#[test]
fn a_filtered_grid_commits_the_model_row_behind_the_display_row() {
    let mut document = support::small_document();
    document.row_filter_mut().visible_rows = Some(vec![2]);
    let mut fixture = EditorFixture::phone(document);
    fixture.run();

    open_cell(&mut fixture, "Carol");
    fixture.input.set_draft("Caroline");
    fixture.click("Done");

    assert_eq!(fixture.cell(2, 0), "Caroline");
    assert_eq!(
        fixture.cell(0, 0),
        "Alice",
        "the display row must not be written as a model row"
    );
    assert_eq!(fixture.cell(1, 0), "Bob");
}

#[test]
fn a_header_tap_clears_the_cell_focus_so_the_next_tap_selects() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    fixture.click_cell("Alice");
    assert_eq!(fixture.state.borrow().focus, Some(GridFocus::new(0, 0)));

    fixture.click_header("Amount");
    assert_eq!(fixture.selection().columns, [1].into_iter().collect());
    assert_eq!(
        fixture.state.borrow().focus,
        None,
        "a touch header selection must not keep a cell border"
    );

    fixture.click_cell("1.5");
    assert!(
        fixture.input.request().is_none(),
        "the next tap selects the row instead of editing the old focus"
    );
    assert_eq!(fixture.selection().rows, [0].into_iter().collect());
    assert_eq!(fixture.edit_target(), None);
}

#[test]
fn a_quoted_multiline_cell_opens_a_multiline_field() {
    let mut fixture = EditorFixture::phone(support::multiline_document());
    fixture.run();

    fixture.click_cell("first\nsecond");
    fixture.click_cell("first\nsecond");

    let request = fixture
        .input
        .request()
        .expect("the multiline cell opens the native editor");
    assert_eq!(request.text, "first\nsecond");
    assert!(
        request.multiline,
        "a cell with an embedded line break must accept line breaks"
    );
}

#[test]
fn a_long_press_opens_the_cell_context_menu_without_selecting() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    let cell = fixture.get_by_role_and_label(Role::Cell, "Alice").rect();
    let pos = cell.center();

    fixture.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos,
        force: Some(1.0),
    });
    fixture.event(egui::Event::PointerMoved(pos));
    fixture.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    fixture.run();
    fixture.run_steps(5);

    assert!(
        fixture.has_label("Copy"),
        "a long press opens the cell context menu"
    );
    assert_eq!(
        fixture.selection().rows,
        [0].into_iter().collect(),
        "the context menu targets the pressed row"
    );
    assert_eq!(
        fixture.edit_target(),
        None,
        "a long press must not run the two-tap edit flow"
    );
    assert!(fixture.input.request().is_none());

    fixture.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    fixture.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::End,
        pos,
        force: None,
    });
    fixture.run();
}

#[test]
fn a_mobile_focus_is_recorded_for_the_second_tap() {
    let mut fixture = EditorFixture::phone(support::small_document());
    fixture.run();
    fixture.click_cell("Bob");

    assert_eq!(
        fixture.state.borrow().focus,
        Some(GridFocus::new(1, 0)),
        "the first tap focuses the tapped cell so the second tap edits it"
    );
}

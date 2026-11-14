//! Fold, resize and keyboard-inset behavior of the touch editor.

use egui::accesskit::Role;
use tabtool_i18n::{Language, tr};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{FoldRegion, MobileEditTarget, MobileMetrics};

use crate::support::{self, EditorFixture, small_document};

const EN: Language = Language::En;

fn open_cell(fixture: &mut EditorFixture, text: &str) {
    fixture.click_cell(text);
    fixture.click_cell(text);
}

#[test]
fn the_action_bar_splits_around_an_active_hinge() {
    let mut fixture = EditorFixture::new(small_document(), [951.0, 800.0], MobileMetrics::phone());
    fixture.set_fold(Some(FoldRegion::new(460.0, 32.0, 44.0)));
    fixture.run();
    fixture.click_header("Name");

    for label in [
        "Add Column Left",
        "Add Column Right",
        "Delete Selection",
        "Sort Ascending",
        "Sort Descending",
        "Remove Sort",
        "Rename Column",
    ] {
        let rect = fixture.get_by_label(tr(EN, label)).rect();
        assert!(
            rect.max.x <= 460.5 || rect.min.x >= 491.5,
            "{label} overlaps the fold at {rect:?}"
        );
        assert!(
            rect.width() >= 44.0 - 0.5,
            "{label} is narrower than 44 pt at {rect:?}"
        );
    }
}

#[test]
fn resizing_keeps_the_document_selection_and_the_open_edit() {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");
    fixture.input.set_draft("Alicia");

    fixture.resize([800.0, 1280.0]);
    fixture.run();
    assert_eq!(fixture.selection().rows, [0].into_iter().collect());
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 0 })
    );
    assert!(fixture.input.request().is_some(), "the session stays open");

    fixture.resize([360.0, 800.0]);
    fixture.run();
    assert_eq!(fixture.selection().rows, [0].into_iter().collect());
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 0, column: 0 })
    );
}

#[test]
fn unfolding_keeps_the_selection_and_the_open_edit() {
    let mut fixture = EditorFixture::new(small_document(), [951.0, 800.0], MobileMetrics::phone());
    fixture.run();
    fixture.click_cell("Bob");
    fixture.click_cell("Bob");

    fixture.set_fold(Some(FoldRegion::new(460.0, 32.0, 44.0)));
    fixture.run();
    fixture.set_fold(None);
    fixture.run();

    assert_eq!(fixture.selection().rows, [1].into_iter().collect());
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 1, column: 0 })
    );
    assert!(fixture.input.request().is_some());
}

#[test]
fn the_action_bar_sits_above_the_status_line() {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_cell("Bob");

    let bar = fixture.get_by_label("Add Row Above").rect();
    let status = fixture
        .query_by_label_contains("rows")
        .expect("the status line reports the rows")
        .rect();
    assert!(
        bar.max.y <= status.min.y + 0.5,
        "the action bar {bar:?} must sit above the status line {status:?}"
    );
}

#[test]
fn the_keyboard_moves_the_accessory_above_the_inset() {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    open_cell(&mut fixture, "Alice");
    fixture.set_keyboard(300.0);
    fixture.run();

    let accessory = fixture.get_by_label("Next Cell").rect();
    let keyboard_top = 800.0 - 300.0;
    assert!(
        accessory.max.y <= keyboard_top + 0.5,
        "the accessory {accessory:?} must sit above the keyboard at {keyboard_top}"
    );
    assert!(
        accessory.width() >= 44.0 - 0.5,
        "every accessory button keeps its touch target"
    );
}

#[test]
fn the_keyboard_hides_the_action_bar() {
    let mut fixture = EditorFixture::phone(small_document());
    fixture.run();
    fixture.click_cell("Bob");
    assert!(
        fixture.has_label("Add Row Above"),
        "a row selection shows the bar"
    );

    fixture.click_cell("Bob");
    fixture.set_keyboard(300.0);
    fixture.run();

    assert!(
        !fixture.has_label("Add Row Above"),
        "the action bar must not compete with the keyboard and accessory"
    );
    assert!(fixture.has_label("Next Cell"));
}

#[test]
fn the_keyboard_never_covers_the_editing_cell() {
    let mut fixture = EditorFixture::phone(support::document(12));
    fixture.run();
    fixture.click_cell("Row 9");
    fixture.click_cell("Row 9");

    fixture.set_keyboard(300.0);
    fixture.run();
    fixture.run();

    let cell = fixture.get_by_role_and_label(Role::Cell, "Row 9").rect();
    let keyboard_top = 800.0 - 300.0;
    assert!(
        cell.max.y <= keyboard_top + 0.5,
        "the editing cell {cell:?} must be scrolled above the keyboard at {keyboard_top}"
    );
    let placed = fixture
        .input
        .placed_rect()
        .expect("the native editor follows the scrolled cell");
    assert!(
        (placed.min.y - cell.min.y).abs() < 1.0 && (placed.max.y - cell.max.y).abs() < 1.0,
        "the native editor {placed:?} must cover the scrolled cell {cell:?}"
    );
    assert_eq!(
        fixture.edit_target(),
        Some(MobileEditTarget::Cell { row: 9, column: 0 })
    );
}

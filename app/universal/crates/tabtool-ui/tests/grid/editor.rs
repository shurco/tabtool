//! The single-cell editor: seeding, commit, move, cancel and focus loss.

use egui::{Key, Role};
use tabtool_ui::grid::{GridAction, GridOptions, GridState};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{GridFixture, sample_table};

fn edit(fixture: &mut GridFixture<'_>, label: &str) {
    fixture.get_by_label(label).click();
    fixture.run();
    fixture.get_by_label(label).click();
    fixture.run();
    fixture.run();
}

fn cell(row: usize, column: usize, text: &str) -> GridAction {
    GridAction::CommitCell {
        row,
        column,
        text: text.to_owned(),
    }
}

#[test]
fn the_second_click_edits_with_the_cell_text_and_the_caret_at_the_end() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");

    let state = fixture.state.borrow();
    let editor = state.editing.as_ref().expect("the cell is being edited");
    assert_eq!((editor.row, editor.column), (1, 0));
    assert_eq!(editor.text, "Bob");
    assert_eq!(editor.caret, 3);
    assert_eq!(state.selection.rows, [1].into_iter().collect());
}

#[test]
fn return_starts_editing_the_focused_cell() {
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
    fixture.key_press(Key::Enter);
    fixture.run();

    let state = fixture.state.borrow();
    let editor = state.editing.as_ref().expect("Return starts the editor");
    assert_eq!((editor.row, editor.column), (0, 0));
    assert_eq!(editor.text, "Alice");
}

#[test]
fn enter_commits_and_moves_down() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");
    fixture.take_actions();

    fixture.key_press(Key::Enter);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![cell(1, 0, "Bob")]);
    let state = fixture.state.borrow();
    assert!(state.editing.is_none());
    assert_eq!(state.selection.rows, [2].into_iter().collect());
    assert_eq!(
        state.focus.map(|focus| (focus.row, focus.column)),
        Some((2, 0))
    );
}

#[test]
fn tab_commits_and_moves_right() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");
    fixture.take_actions();

    fixture.key_press(Key::Tab);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![cell(1, 0, "Bob")]);
    let state = fixture.state.borrow();
    assert!(state.editing.is_none());
    assert_eq!(state.selection.rows, [1].into_iter().collect());
    assert_eq!(
        state.focus.map(|focus| (focus.row, focus.column)),
        Some((1, 1))
    );
}

#[test]
fn tab_on_the_last_column_wraps_to_the_next_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "first");
    fixture.take_actions();

    fixture.key_press(Key::Tab);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![cell(0, 2, "first")]);
    let state = fixture.state.borrow();
    assert_eq!(
        state.focus.map(|focus| (focus.row, focus.column)),
        Some((1, 0))
    );
    assert_eq!(state.selection.rows, [1].into_iter().collect());
}

#[test]
fn escape_cancels_without_an_action() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");
    fixture.take_actions();

    fixture.key_press(Key::Escape);
    fixture.run();

    assert!(fixture.take_actions().is_empty());
    let state = fixture.state.borrow();
    assert!(state.editing.is_none());
    assert_eq!(state.selection.rows, [1].into_iter().collect());
}

#[test]
fn focus_loss_commits_exactly_once() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");
    fixture.take_actions();

    fixture.get_by_label("Carol").click();
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![cell(1, 0, "Bob")]);
    assert!(fixture.state.borrow().editing.is_none());
    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());

    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "the editor committed twice"
    );
}

#[test]
fn typing_changes_the_committed_text() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Bob");
    fixture.take_actions();

    {
        let field = fixture.get_by_role(Role::TextInput);
        field.type_text("!");
    }
    fixture.run();
    assert_eq!(
        fixture.state.borrow().editing.as_ref().unwrap().text,
        "Bob!"
    );

    fixture.key_press(Key::Enter);
    fixture.run();
    assert_eq!(fixture.take_actions(), vec![cell(1, 0, "Bob!")]);
}

#[test]
fn enter_on_the_last_row_stays_on_the_last_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    edit(&mut fixture, "Dave");
    fixture.take_actions();

    fixture.key_press(Key::Enter);
    fixture.run();

    assert_eq!(fixture.take_actions(), vec![cell(3, 0, "Dave")]);
    let state = fixture.state.borrow();
    assert_eq!(
        state.focus.map(|focus| (focus.row, focus.column)),
        Some((3, 0))
    );
    assert_eq!(state.selection.rows, [3].into_iter().collect());
}

#[test]
fn a_read_only_grid_never_starts_an_editor() {
    let source = sample_table();
    let options = GridOptions {
        editable: false,
        ..GridOptions::default()
    };
    let mut fixture = GridFixture::new(&source, GridState::default(), options, [640.0, 400.0]);
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    assert!(fixture.take_actions().is_empty());
    assert!(fixture.state.borrow().editing.is_none());

    fixture.key_press(Key::Enter);
    fixture.run();
    assert!(fixture.state.borrow().editing.is_none());
}

#[test]
fn a_row_number_click_focuses_the_row_for_return() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("3").click();
    fixture.run();
    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());

    fixture.key_press(Key::Enter);
    fixture.run();

    let state = fixture.state.borrow();
    let editor = state.editing.as_ref().expect("Return starts the editor");
    assert_eq!((editor.row, editor.column), (2, 0));
    assert_eq!(editor.text, "Carol");
}

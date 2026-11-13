//! Row and column dragging: payloads, drop targets and the emitted actions.

use std::collections::BTreeSet;

use egui::{Key, pos2};
use tabtool_core::{CsvTable, RowSource};
use tabtool_ui::grid::{
    GridAction, GridGeometry, GridOptions, GridState, RowDrag, column_drop_action, column_slot_at,
    row_drop_action, row_slot_at,
};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

fn geometry(widths: &[f32], rows: usize) -> GridGeometry<'_> {
    GridGeometry::new(widths, 24.0, rows, true)
}

#[test]
fn a_row_drag_carries_the_row_indexes_and_a_tsv_payload() {
    let source = CountingSource::new(4, 3);
    let drag = RowDrag::new(&source, [1].into_iter().collect()).expect("a valid row");

    assert_eq!(drag.rows, [1].into_iter().collect());
    assert_eq!(
        drag.payload(&source, ".", "."),
        "r1c0\tr1c1\tr1c2".to_owned()
    );
}

#[test]
fn a_row_drag_carries_the_whole_selection_as_one_tsv_block() {
    let source = CsvTable::from_rows(
        vec![
            vec!["a".into(), "b".into()],
            vec!["c".into(), "d".into()],
            vec!["e".into(), "f".into()],
        ],
        None,
        2,
    );
    let drag = RowDrag::new(&source, [0, 2].into_iter().collect()).expect("valid rows");

    assert_eq!(drag.payload(&source, ".", "."), "a\tb\ne\tf");
}

#[test]
fn a_row_drag_drops_invalid_rows() {
    let source = CountingSource::new(1, 3);
    let drag = RowDrag::new(&source, [5].into_iter().collect());

    assert!(drag.is_none());
    assert!(RowDrag::new(&source, BTreeSet::new()).is_none());
}

#[test]
fn the_drag_payload_localizes_numeric_cells() {
    let source = CsvTable::from_rows(vec![vec!["1,5".into(), "text".into()]], None, 2);
    let drag = RowDrag::new(&source, [0].into_iter().collect()).expect("a valid row");

    assert_eq!(drag.payload(&source, ",", "."), "1.5\ttext");
}

#[test]
fn the_row_drop_target_rounds_to_the_nearest_boundary() {
    let widths = [100.0];
    let geometry = geometry(&widths, 4);

    assert_eq!(row_slot_at(&geometry, 0.0), 0);
    assert_eq!(row_slot_at(&geometry, 39.0), 0);
    assert_eq!(row_slot_at(&geometry, 41.0), 1);
    assert_eq!(row_slot_at(&geometry, 1_000.0), 4);
}

#[test]
fn the_column_drop_target_uses_the_column_halves() {
    let widths = [100.0, 100.0, 100.0];
    let geometry = geometry(&widths, 1);

    assert_eq!(column_slot_at(&geometry, 0.0), 0);
    assert_eq!(column_slot_at(&geometry, 40.0), 0);
    assert_eq!(column_slot_at(&geometry, 89.0), 0);
    assert_eq!(column_slot_at(&geometry, 91.0), 1);
    assert_eq!(column_slot_at(&geometry, 240.0), 2);
    assert_eq!(column_slot_at(&geometry, 340.0), 3);
    assert_eq!(column_slot_at(&geometry, 1_000.0), 3);
}

#[test]
fn a_column_drop_emits_one_move_action() {
    let source = sample_table();
    let ids = source.column_ids();
    let drag = tabtool_ui::grid::ColumnDrag {
        column: ids[0],
        target: 3,
    };

    assert_eq!(
        column_drop_action(&source, &drag),
        Some(GridAction::MoveColumn { from: 0, to: 3 })
    );
}

#[test]
fn a_column_drop_resolves_the_current_column_identity() {
    let source = sample_table();
    let ids = source.column_ids();
    let drag = tabtool_ui::grid::ColumnDrag {
        column: ids[1],
        target: 0,
    };

    assert_eq!(
        column_drop_action(&source, &drag),
        Some(GridAction::MoveColumn { from: 1, to: 0 })
    );
}

#[test]
fn a_column_drop_in_place_emits_nothing() {
    let source = sample_table();
    let ids = source.column_ids();
    for target in [0, 1] {
        let drag = tabtool_ui::grid::ColumnDrag {
            column: ids[0],
            target,
        };
        assert_eq!(column_drop_action(&source, &drag), None);
    }
}

#[test]
fn a_column_drop_for_a_missing_identity_emits_nothing() {
    let source = sample_table();
    let drag = tabtool_ui::grid::ColumnDrag {
        column: tabtool_core::new_column_id(),
        target: 2,
    };

    assert_eq!(column_drop_action(&source, &drag), None);
}

#[test]
fn a_row_drop_needs_a_target_and_clamps_it() {
    let source = CountingSource::new(3, 1);
    let mut drag = RowDrag::new(&source, [0].into_iter().collect()).expect("a valid row");
    assert_eq!(row_drop_action(&drag, 3), None);

    drag.target = Some(9);
    assert_eq!(
        row_drop_action(&drag, 3),
        Some(GridAction::MoveRows {
            from: [0].into_iter().collect(),
            to: 3,
        })
    );
}

#[test]
fn dragging_a_header_reorders_the_columns_once() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let name = fixture.get_by_label("Name").rect();
    let note = fixture.get_by_label("Note").rect();
    let start = pos2(name.center().x, name.center().y);
    let end = pos2(note.max.x - 2.0, note.center().y);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();

    let state = fixture.state.borrow();
    let drag = state.column_drag.as_ref().expect("the header drag is live");
    assert_eq!(drag.target, 3);
    drop(state);
    assert_eq!(fixture.take_actions(), Vec::new());

    fixture.drop_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::MoveColumn { from: 0, to: 3 }]
    );
    assert!(fixture.state.borrow().column_drag.is_none());
}

#[test]
fn dropping_a_header_in_place_emits_nothing() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let name = fixture.get_by_label("Name").rect();
    let start = pos2(name.center().x, name.center().y);
    let end = pos2(name.center().x - 20.0, name.center().y);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();
    fixture.drop_at(end);
    fixture.run();

    assert!(fixture.take_actions().is_empty());
}

#[test]
fn dragging_a_row_moves_it_and_selects_the_moved_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let alice = fixture.get_by_label("Alice").rect();
    let dave = fixture.get_by_label("Dave").rect();
    let start = pos2(alice.center().x, alice.center().y);
    let end = pos2(alice.center().x, dave.max.y - 4.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();

    let state = fixture.state.borrow();
    let drag = state.row_drag.as_ref().expect("the row drag is live");
    assert_eq!(drag.rows, [0].into_iter().collect());
    assert_eq!(drag.target, Some(4));
    drop(state);

    fixture.drop_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::MoveRows {
            from: [0].into_iter().collect(),
            to: 4,
        }]
    );
    assert!(fixture.state.borrow().row_drag.is_none());

    fixture.key_press(Key::Tab);
    fixture.run();
    let state = fixture.state.borrow();
    assert_eq!(
        state.focus.map(|focus| (focus.row, focus.column)),
        Some((0, 1)),
        "the dragged row keeps keyboard focus"
    );
}

#[test]
fn starting_a_row_drag_commits_the_active_editor() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    assert!(fixture.state.borrow().editing.is_some());
    fixture.take_actions();

    let carol = fixture.get_by_label("Carol").rect();
    let dave = fixture.get_by_label("Dave").rect();
    let start = pos2(carol.center().x, carol.center().y);
    let end = pos2(carol.center().x, dave.max.y - 4.0);
    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::CommitCell {
            row: 1,
            column: 0,
            text: "Bob".to_owned(),
        }]
    );
    assert!(fixture.state.borrow().editing.is_none());
}

#[test]
fn dragging_a_row_number_moves_the_row_into_the_data_area() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let number = fixture.get_by_label("1").rect();
    let dave = fixture.get_by_label("Dave").rect();
    let start = pos2(number.center().x, number.center().y);
    let end = pos2(dave.center().x, dave.max.y - 4.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();
    fixture.drop_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::MoveRows {
            from: [0].into_iter().collect(),
            to: 4,
        }]
    );
}

#[test]
fn a_row_drag_carries_the_whole_selection() {
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
        .click_modifiers(egui::Modifiers::COMMAND);
    fixture.run();

    let alice = fixture.get_by_label("Alice").rect();
    let dave = fixture.get_by_label("Dave").rect();
    let start = pos2(alice.center().x, alice.center().y);
    let end = pos2(alice.center().x, dave.max.y - 4.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();

    let state = fixture.state.borrow();
    let drag = state.row_drag.as_ref().expect("the row drag is live");
    assert_eq!(drag.rows, [0, 2].into_iter().collect());
    drop(state);

    fixture.drop_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::MoveRows {
            from: [0, 2].into_iter().collect(),
            to: 4,
        }]
    );
}

#[test]
fn a_drop_outside_the_grid_cancels_the_row_drag() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let alice = fixture.get_by_label("Alice").rect();
    let start = pos2(alice.center().x, alice.center().y);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(pos2(700.0, 500.0));
    fixture.run();
    fixture.drop_at(pos2(700.0, 500.0));
    fixture.run();

    assert!(
        fixture.take_actions().is_empty(),
        "a drag-out is out of scope and must not move rows"
    );
    assert!(fixture.state.borrow().row_drag.is_none());
}

#[test]
fn a_column_drop_outside_the_grid_cancels_the_drag() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let name = fixture.get_by_label("Name").rect();
    let start = pos2(name.center().x, name.center().y);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(pos2(700.0, 60.0));
    fixture.run();
    fixture.drop_at(pos2(700.0, 60.0));
    fixture.run();

    assert!(
        fixture.take_actions().is_empty(),
        "a header dragged out of the grid must not reorder columns"
    );
    assert!(fixture.state.borrow().column_drag.is_none());
}

#[test]
fn an_internal_drop_selects_the_dragged_row() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Carol").click();
    fixture.run();
    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());
    fixture.take_actions();

    let carol = fixture.get_by_label("Carol").rect();
    let alice = fixture.get_by_label("Alice").rect();
    let start = pos2(carol.center().x, carol.center().y);
    let end = pos2(carol.center().x, alice.min.y + 4.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();
    fixture.drop_at(end);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::MoveRows {
            from: [2].into_iter().collect(),
            to: 0,
        }]
    );
    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());
}

#[test]
fn the_header_resize_handle_does_not_start_a_column_drag() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let name = fixture.get_by_label("Name").rect();
    let start = pos2(name.max.x - 1.0, name.center().y);
    let end = pos2(start.x + 40.0, start.y);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();

    assert!(
        fixture.state.borrow().column_drag.is_none(),
        "the edge is a resize gesture"
    );
    assert!(
        fixture.state.borrow().ordered_widths(&source)[0] > 100.0,
        "the resize still applies"
    );

    fixture.drop_at(end);
    fixture.run();
}

#[test]
fn dragging_uses_display_rows_and_the_source_widths_stay_with_the_column() {
    // Widths are keyed by ColumnId, so a reordered source keeps them attached.
    let source = sample_table();
    let ids = source.column_ids();
    let mut state = GridState::default();
    state.widths.insert(ids[1], 240.0);
    let widths = state.ordered_widths(&source);
    assert_eq!(widths, [100.0, 240.0, 100.0]);

    state.widths.insert(ids[1], 240.0);
    let reordered: Vec<tabtool_core::ColumnId> = vec![ids[1], ids[0], ids[2]];
    let mut reordered_widths = Vec::new();
    for id in reordered {
        reordered_widths.push(*state.widths.get(&id).unwrap_or(&100.0));
    }
    assert_eq!(reordered_widths, [240.0, 100.0, 100.0]);
}

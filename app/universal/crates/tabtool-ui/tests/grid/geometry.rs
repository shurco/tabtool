//! Geometry, appearance and state fitting for the grid.

use egui::{Rect, pos2, vec2};
use tabtool_core::{RowSource, new_column_id};
use tabtool_ui::grid::{DEFAULT_COLUMN_WIDTH, GridGeometry, GridOptions, GridState};

use crate::support::{CountingSource, sample_table};

#[test]
fn first_cell_sits_below_the_header_and_beside_the_gutter() {
    let widths = [DEFAULT_COLUMN_WIDTH; 3];
    let geometry = GridGeometry::new(&widths, 24.0, 2, true);

    assert_eq!(
        geometry.cell_rect(0, 0),
        Rect::from_min_size(pos2(40.0, 28.0), vec2(100.0, 24.0))
    );
    assert_eq!(geometry.content_size(), vec2(40.0 + 300.0, 28.0 + 48.0));
}

#[test]
fn hiding_row_numbers_removes_the_gutter() {
    let widths = [DEFAULT_COLUMN_WIDTH];
    let geometry = GridGeometry::new(&widths, 24.0, 1, false);

    assert_eq!(geometry.cell_rect(0, 0).min.x, 0.0);
    assert_eq!(geometry.content_size().x, 100.0);
    assert_eq!(geometry.gutter(), 0.0);
}

#[test]
fn pinned_header_and_gutter_follow_the_visible_rect() {
    let widths = [DEFAULT_COLUMN_WIDTH; 8];
    let geometry = GridGeometry::new(&widths, 24.0, 20, true);
    let viewport = Rect::from_min_size(pos2(60.0, 40.0), vec2(300.0, 400.0));

    assert_eq!(
        geometry.pinned_header_rect(1, viewport),
        Rect::from_min_size(pos2(40.0 + 100.0 - 60.0, 0.0), vec2(100.0, 28.0))
    );
    assert_eq!(
        geometry.pinned_gutter_rect(3, viewport),
        Rect::from_min_size(pos2(0.0, 28.0 + 72.0 - 40.0), vec2(40.0, 24.0))
    );
    assert_eq!(geometry.pinned_corner_rect().min, pos2(0.0, 0.0));
}

#[test]
fn column_widths_come_from_the_geometry() {
    let widths = [100.0, 200.0, 300.0];
    let geometry = GridGeometry::new(&widths, 24.0, 1, true);

    assert_eq!(geometry.column_x(0), 40.0);
    assert_eq!(geometry.column_x(1), 140.0);
    assert_eq!(geometry.column_x(2), 340.0);
    assert_eq!(geometry.column_width(2), 300.0);
    assert_eq!(geometry.content_size().x, 40.0 + 600.0);
}

#[test]
fn visible_rows_add_one_row_of_overscan() {
    let widths = [160.0];
    let geometry = GridGeometry::new(&widths, 24.0, 100, true);
    let top = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
    let scrolled = Rect::from_min_size(pos2(0.0, 1000.0), vec2(300.0, 400.0));

    assert_eq!(geometry.visible_rows(top), 0..17);
    assert_eq!(geometry.visible_rows(scrolled), 40..59);
}

#[test]
fn visible_rows_never_pass_the_last_row() {
    let widths = [160.0];
    let geometry = GridGeometry::new(&widths, 24.0, 20, true);
    let past_the_end = Rect::from_min_size(pos2(0.0, 1000.0), vec2(300.0, 400.0));
    let rows = geometry.visible_rows(past_the_end);

    assert!(rows.end <= 20, "rows {rows:?} passed the row count");
    assert!(rows.start <= rows.end);
}

#[test]
fn visible_columns_add_overscan_and_clip_to_the_count() {
    let widths = [100.0, 200.0, 300.0];
    let geometry = GridGeometry::new(&widths, 24.0, 10, true);
    let top = Rect::from_min_size(pos2(0.0, 0.0), vec2(250.0, 400.0));
    let scrolled = Rect::from_min_size(pos2(140.0, 0.0), vec2(250.0, 400.0));

    assert_eq!(geometry.visible_columns(top), 0..3);
    assert_eq!(geometry.visible_columns(scrolled), 0..3);
}

#[test]
fn hit_testing_skips_the_pinned_header_and_gutter() {
    let widths = [160.0, 160.0];
    let geometry = GridGeometry::new(&widths, 24.0, 10, true);

    assert_eq!(geometry.cell_at(pos2(50.0, 30.0)), Some((0, 0)));
    assert_eq!(geometry.cell_at(pos2(210.0, 70.0)), Some((1, 1)));
    assert_eq!(geometry.cell_at(pos2(10.0, 50.0)), None);
    assert_eq!(geometry.cell_at(pos2(50.0, 10.0)), None);
    assert_eq!(geometry.cell_at(pos2(400.0, 50.0)), None);
}

#[test]
fn row_numbers_are_one_based_and_include_the_header_offset() {
    let widths = [160.0];
    let geometry = GridGeometry::new(&widths, 24.0, 2, true);
    let offset = GridGeometry {
        header_offset: 1,
        ..geometry
    };

    assert_eq!(geometry.row_number(0), 1);
    assert_eq!(geometry.row_number(2), 3);
    assert_eq!(offset.row_number(0), 2);
    assert_eq!(offset.row_number(2), 4);
    let options = GridOptions {
        header_offset: 1,
        ..GridOptions::default()
    };
    assert_eq!(options.physical_row(0), 1);
    assert_eq!(options.physical_row(2), 3);
}

#[test]
fn the_gutter_grows_for_long_row_numbers() {
    let widths = [160.0];
    let small = GridGeometry::new(&widths, 24.0, 2, true);
    let large = GridGeometry::new(&widths, 24.0, 1_000_000, true);

    assert_eq!(small.gutter(), 40.0);
    assert!(large.gutter() > 40.0);
}

#[test]
fn alternating_rows_replace_the_horizontal_grid_lines() {
    let plain = GridOptions {
        alternating_rows: false,
        ..GridOptions::default()
    };
    let alternating = GridOptions {
        alternating_rows: true,
        ..GridOptions::default()
    };

    assert!(plain.shows_horizontal_lines());
    assert!(!alternating.shows_horizontal_lines());
    assert_eq!(GridOptions::default().row_height, 24.0);
    assert_eq!(GridOptions::default().default_column_width, 100.0);
    assert!(GridOptions::default().max_column_width.is_infinite());
}

#[test]
fn the_row_height_setting_reaches_the_geometry() {
    for row_height in [20.0, 24.0, 32.0] {
        let widths = [160.0];
        let geometry = GridGeometry::new(&widths, row_height, 3, true);
        assert_eq!(geometry.content_size().y, 28.0 + 3.0 * row_height);
    }
}

#[test]
fn resizing_a_column_clamps_to_the_minimum_and_has_no_default_cap() {
    let source = sample_table();
    let ids = source.column_ids();
    let options = GridOptions::default();
    let mut state = GridState::default();
    state.sync(&source, &options);

    assert_eq!(state.ordered_widths(&source), vec![DEFAULT_COLUMN_WIDTH; 3]);
    state.resize_column(&source, 0, -10_000.0, &options);
    assert_eq!(state.widths[&ids[0]], options.min_column_width);
    state.resize_column(&source, 0, 10_000.0, &options);
    assert_eq!(state.widths[&ids[0]], 40.0 + 10_000.0);
}

#[test]
fn a_new_source_keeps_scroll_selection_and_widths() {
    let options = GridOptions::default();
    let first = CountingSource::new(10, 3);
    let mut ids = first.ids().to_vec();
    let mut state = GridState::default();
    state.sync(&first, &options);
    state.widths.insert(ids[0], 120.0);
    state.widths.insert(ids[1], 180.0);
    state.widths.insert(ids[2], 240.0);
    state.scroll = vec2(5.0, 240.0);
    state.selection.rows.insert(2);
    state.focus = Some(tabtool_ui::grid::GridFocus::new(2, 1));

    ids.extend([new_column_id(), new_column_id()]);
    let second = CountingSource::with_ids(50, ids.clone());
    state.sync(&second, &options);

    assert_eq!(&state.ordered_widths(&second)[..3], &[120.0, 180.0, 240.0]);
    assert_eq!(state.widths.len(), 5);
    assert_eq!(state.ordered_widths(&second)[3], DEFAULT_COLUMN_WIDTH);
    assert_eq!(state.scroll, vec2(5.0, 240.0));
    assert_eq!(
        state.selection.rows.iter().copied().collect::<Vec<_>>(),
        [2]
    );

    let smaller = CountingSource::with_ids(1, ids[..2].to_vec());
    state.sync(&smaller, &options);
    assert_eq!(state.ordered_widths(&smaller), vec![120.0, 180.0]);
    assert!(state.selection.rows.is_empty());
    assert_eq!(state.focus, None);
}

#[test]
fn a_column_move_keeps_widths_with_their_column_identity() {
    let options = GridOptions::default();
    let first = CountingSource::new(10, 3);
    let ids = first.ids().to_vec();
    let mut state = GridState::default();
    state.sync(&first, &options);
    state.widths.insert(ids[0], 120.0);
    state.widths.insert(ids[1], 180.0);
    state.widths.insert(ids[2], 240.0);

    let moved = CountingSource::with_ids(10, vec![ids[2], ids[0], ids[1]]);
    state.sync(&moved, &options);

    assert_eq!(state.ordered_widths(&moved), vec![240.0, 120.0, 180.0]);
}

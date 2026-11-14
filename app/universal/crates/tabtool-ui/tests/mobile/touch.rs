//! Touch gesture resolution, focus-beats-selection highlighting and the touch
//! grid options.

use tabtool_core::CsvConfiguration;
use tabtool_i18n::Language;
use tabtool_ui::document::grid_options;
use tabtool_ui::grid::{
    GridAppearance, GridElement, GridFocus, GridHighlight, GridOptions, GridSelection, GridTouch,
    TouchGesture,
};
use tabtool_ui::mobile::{MOBILE_DEFAULT_ROW_HEIGHT, MOBILE_ROW_HEIGHTS};

const EN: Language = Language::En;

fn document() -> tabtool_document::Document {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = tabtool_document::Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"Name,Amount\nAlice,1.5\nBob,20\n",
            None,
            Some(configuration),
            false,
        )
        .expect("the fixture parses");
    document
}

fn cell() -> GridFocus {
    GridFocus::new(2, 3)
}

fn selection(rows: &[usize], columns: &[usize]) -> GridSelection {
    GridSelection {
        rows: rows.iter().copied().collect(),
        columns: columns.iter().copied().collect(),
    }
}

#[test]
fn first_tap_selects_the_row() {
    let touch = GridTouch::new();
    assert_eq!(
        touch.tap(GridElement::Cell, None, Some(cell())),
        TouchGesture::SelectCell
    );
}

#[test]
fn a_second_tap_on_the_focused_cell_edits() {
    let touch = GridTouch::new();
    assert_eq!(
        touch.tap(GridElement::Cell, Some(cell()), Some(cell())),
        TouchGesture::EditCell
    );
}

#[test]
fn a_tap_on_another_cell_selects_instead_of_editing() {
    let touch = GridTouch::new();
    let other = GridFocus::new(2, 4);
    assert_eq!(
        touch.tap(GridElement::Cell, Some(cell()), Some(other)),
        TouchGesture::SelectCell
    );
}

#[test]
fn header_and_row_number_select_their_axis() {
    let touch = GridTouch::new();
    assert_eq!(
        touch.tap(GridElement::ColumnHeader, Some(cell()), None),
        TouchGesture::SelectColumn
    );
    assert_eq!(
        touch.tap(GridElement::RowNumber, Some(cell()), None),
        TouchGesture::SelectRowNumber
    );
}

#[test]
fn corner_and_empty_hits_are_ignored() {
    let touch = GridTouch::new();
    assert_eq!(
        touch.tap(GridElement::Corner, Some(cell()), None),
        TouchGesture::Ignore
    );
    assert_eq!(
        touch.tap(GridElement::Cell, None, None),
        TouchGesture::Ignore
    );
}

#[test]
fn a_long_press_opens_the_context_menu() {
    let touch = GridTouch::new();
    for element in [
        GridElement::Cell,
        GridElement::ColumnHeader,
        GridElement::RowNumber,
    ] {
        assert_eq!(
            touch.long_press(element),
            TouchGesture::ContextMenu,
            "{element:?} must offer its context menu"
        );
    }
    assert_eq!(touch.long_press(GridElement::Corner), TouchGesture::Ignore);
}

#[test]
fn a_double_tap_is_reserved_for_zoom_and_never_edits() {
    let touch = GridTouch::new();
    assert_eq!(touch.double_tap(), TouchGesture::Ignore);
}

#[test]
fn focus_beats_selection() {
    let touch = GridTouch::new();
    assert_eq!(
        touch.highlight(&selection(&[0], &[]), Some(GridFocus::new(0, 0)), 0, 0),
        GridHighlight::Focused
    );
}

#[test]
fn a_selected_row_highlights_every_column() {
    let touch = GridTouch::new();
    let selected = selection(&[0], &[]);
    assert_eq!(
        touch.highlight(&selected, Some(GridFocus::new(0, 0)), 0, 1),
        GridHighlight::Selected
    );
    assert_eq!(
        touch.highlight(&selected, Some(GridFocus::new(0, 0)), 1, 1),
        GridHighlight::None
    );
}

#[test]
fn a_selected_column_highlights_every_row() {
    let touch = GridTouch::new();
    let selected = selection(&[], &[1]);
    assert_eq!(
        touch.highlight(&selected, None, 1, 1),
        GridHighlight::Selected
    );
    assert_eq!(touch.highlight(&selected, None, 1, 0), GridHighlight::None);
}

#[test]
fn the_touch_options_use_touch_rows_and_drop_hover_and_keys() {
    let document = document();
    let options = grid_options(&document, EN, GridAppearance::default(), true)
        .for_touch(MOBILE_DEFAULT_ROW_HEIGHT);

    assert_eq!(options.row_height, MOBILE_DEFAULT_ROW_HEIGHT);
    assert!(MOBILE_ROW_HEIGHTS.contains(&options.row_height));
    assert!(options.touch, "the grid must know it is driven by touch");
    assert!(
        !options.keys_enabled,
        "hardware keys must not open the egui cell editor"
    );
    assert!(options.editable, "an in-memory document is editable");
    assert_eq!(options.header_offset, 0);
    assert_eq!(options.default_column_width, 100.0);
    assert!(options.max_column_width >= options.min_column_width);
}

#[test]
fn each_touch_row_height_is_accepted_verbatim() {
    for height in MOBILE_ROW_HEIGHTS {
        let options = GridOptions::default().for_touch(height);
        assert_eq!(options.row_height, height);
    }
}

#[test]
fn the_document_capabilities_follow_a_large_filtered_view() {
    let configuration = CsvConfiguration::default();
    let mut document = tabtool_document::Document::new(configuration.clone());
    document
        .load_from_bytes(b"a,b\n1,2\n3,4\n", None, Some(configuration), false)
        .expect("the fixture parses");

    let options = grid_options(&document, EN, GridAppearance::default(), false);
    assert!(!options.keys_enabled);
    assert_eq!(options.header_offset, 0);
    assert_eq!(
        options.menu,
        tabtool_ui::grid::GridMenuOptions::of(&document)
    );
}

#[test]
fn large_mode_offsets_the_header_and_disables_select_all() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, "Name,Amount\nAlice,1\nBob,2\n").expect("the fixture is written");
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = tabtool_document::Document::new(configuration.clone());
    document
        .open_large(&path, Some(configuration), false, 0)
        .expect("the large fixture opens");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();

    let options = grid_options(&document, EN, GridAppearance::default(), true).for_touch(44.0);
    assert!(document.is_large_mode());
    assert_eq!(
        options.header_offset, 1,
        "the physical header row moves the gutter numbering"
    );
    assert!(!document.can_select_all(), "a large file has no Select All");
    assert!(!options.keys_enabled, "touch editing does not read keys");
    assert!(options.editable, "a ready large file edits cells");
}

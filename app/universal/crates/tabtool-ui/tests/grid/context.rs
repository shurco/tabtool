//! Context-menu models, enablement and the inline header rename editor.

use egui::{Event, ImeEvent, Key, Role};
use tabtool_core::{CsvConfiguration, FilterCriterion, FilterOperator, RowSource, SortDirection};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_ui::grid::{
    GridAction, GridMenuOptions, GridOptions, GridSelection, GridState, MenuCommand, MenuEntry,
    cell_menu, header_menu, row_menu,
};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

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

fn loaded(csv: &str) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

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
    document
}

fn large_document() -> (tempfile::TempDir, Document) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"h1,h2\na,b\nc,d\n").expect("write the fixture");
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(&path, None, false, 0)
        .expect("open the fixture large");
    document.large_state().expect("large state").indexer.wait();
    document.poll_large_indexing();
    (directory, document)
}

fn labels(menu: &[MenuEntry]) -> Vec<&'static str> {
    menu.iter().map(|entry| entry.label).collect()
}

fn enabled(menu: &[MenuEntry], label: &str) -> bool {
    menu.iter()
        .find(|entry| entry.label == label)
        .expect("the menu has the entry")
        .enabled
}

fn default_options() -> GridMenuOptions {
    GridMenuOptions::default()
}

fn grid_with_options<'a>(
    source: &'a dyn tabtool_core::RowSource,
    options: GridOptions,
) -> GridFixture<'a> {
    GridFixture::new(source, GridState::default(), options, [640.0, 400.0])
}

#[test]
fn the_header_menu_lists_the_mac_commands_in_order() {
    let entries = header_menu(&default_options(), 0, &Default::default(), 3);
    assert_eq!(
        labels(&entries),
        [
            "Sort Ascending",
            "Sort Descending",
            "Remove Sort",
            "",
            "Add Column Left",
            "Add Column Right",
            "",
            "Delete Column(s)",
            "",
            "Rename Column",
        ]
    );
    assert_eq!(
        entries[0].command,
        Some(MenuCommand::Run(GridAction::Sort {
            column: 0,
            direction: SortDirection::Ascending,
        }))
    );
}

#[test]
fn the_row_menu_adds_and_deletes_rows() {
    let mut selection = Default::default();
    let entries = row_menu(&default_options(), &selection);
    assert_eq!(
        labels(&entries),
        ["Add Row Above", "Add Row Below", "", "Delete Row(s)"]
    );
    assert!(!enabled(&entries, "Delete Row(s)"));

    selection.rows.insert(1);
    let entries = row_menu(&default_options(), &selection);
    assert!(enabled(&entries, "Delete Row(s)"));
}

#[test]
fn the_cell_menu_has_copy_paste_and_select_all() {
    let entries = cell_menu(&default_options(), &Default::default(), 2, 1);
    assert_eq!(labels(&entries), ["Copy", "Paste", "", "Select All"]);
    assert!(!enabled(&entries, "Copy"));
    assert!(enabled(&entries, "Paste"));
    assert!(enabled(&entries, "Select All"));
}

#[test]
fn select_all_is_disabled_for_a_large_document() {
    let options = GridMenuOptions {
        large: true,
        ..GridMenuOptions::default()
    };
    let menu = cell_menu(&options, &Default::default(), 100, 3);

    assert!(!enabled(&menu, "Select All"));
}

#[test]
fn structural_items_disable_together() {
    let options = GridMenuOptions {
        structural: false,
        row_structural: false,
        can_remove_sort: false,
        large: false,
    };
    let mut selection: GridSelection = Default::default();
    selection.columns.insert(0);
    selection.rows.insert(0);
    let header = header_menu(&options, 0, &selection, 2);
    assert!(!enabled(&header, "Sort Ascending"));
    assert!(
        !header
            .iter()
            .any(|entry| entry.label == "Add Column Left" && entry.enabled)
    );
    assert!(!enabled(&header, "Rename Column"));
    let row = row_menu(&options, &selection);
    assert!(!enabled(&row, "Add Row Above"));
    assert!(!enabled(&row, "Delete Row(s)"));
    let cell = cell_menu(&options, &Default::default(), 2, 2);
    assert!(!enabled(&cell, "Paste"));
}

#[test]
fn header_delete_needs_a_column_selection_and_another_column() {
    let mut selection = Default::default();
    let menu = header_menu(&default_options(), 0, &selection, 2);
    assert!(!enabled(&menu, "Delete Column(s)"));

    selection.columns.insert(0);
    let single = header_menu(&default_options(), 0, &selection, 1);
    assert!(!enabled(&single, "Delete Column(s)"));

    let menu = header_menu(&default_options(), 0, &selection, 2);
    assert!(enabled(&menu, "Delete Column(s)"));
}

#[test]
fn rename_is_only_enabled_for_an_existing_column() {
    let menu = header_menu(&default_options(), 5, &Default::default(), 2);
    assert!(!enabled(&menu, "Rename Column"));
}

#[test]
fn remove_sort_follows_the_active_sort() {
    let mut options = default_options();
    options.can_remove_sort = false;
    assert!(!enabled(
        &header_menu(&options, 0, &Default::default(), 1),
        "Remove Sort"
    ));

    options.can_remove_sort = true;
    assert!(enabled(
        &header_menu(&options, 0, &Default::default(), 1),
        "Remove Sort"
    ));
}

#[test]
fn menu_labels_come_from_the_i18n_catalog() {
    let entries = header_menu(&default_options(), 0, &Default::default(), 3);
    assert_eq!(entries[0].text(Language::En), "Sort Ascending");
    assert_eq!(entries[0].text(Language::Fr), "Trier par ordre croissant");
    assert_eq!(entries[4].text(Language::De), "Spalte links hinzufügen");
}

#[test]
fn menu_options_follow_the_document() {
    let ready = loaded("a,b\n1,2\n");
    let options = GridMenuOptions::of(&ready);
    assert!(options.structural);
    assert!(options.row_structural);
    assert!(!options.can_remove_sort);
    assert!(!options.large);

    let filtered = filtered_document();
    let options = GridMenuOptions::of(&filtered);
    assert!(options.structural);
    assert!(!options.row_structural);

    let (_directory, large) = large_document();
    let options = GridMenuOptions::of(&large);
    assert!(!options.structural, "large files have no structural edits");
    assert!(!options.row_structural);
    assert!(options.large);

    let loading = Document::new(CsvConfiguration::default());
    let options = GridMenuOptions::of(&loading);
    assert!(!options.structural);
    assert!(!options.row_structural);
}

#[test]
fn menu_options_report_an_active_sort() {
    let mut document = header_document("h\nb\na\n");
    document.sort_rows(
        0,
        SortDirection::Ascending,
        &"en-US".parse().expect("locale"),
    );

    assert!(GridMenuOptions::of(&document).can_remove_sort);
}

// ---------------------------------------------------------------------------
// Rendered context menus
// ---------------------------------------------------------------------------

#[test]
fn right_click_on_a_header_opens_the_column_menu_and_selects_it() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Amount").click_secondary();
    fixture.run();
    fixture.run();

    assert_eq!(fixture.selected_columns(), [1].into_iter().collect());
    assert!(fixture.selected_rows().is_empty());
    fixture.get_by_label("Sort Ascending");
    fixture.get_by_label("Rename Column");

    fixture.get_by_label("Sort Ascending").click();
    fixture.run();
    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::Sort {
            column: 1,
            direction: SortDirection::Ascending,
        }]
    );
}

#[test]
fn a_header_menu_keeps_an_existing_column_selection() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.columns = [0, 2].into_iter().collect();
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.get_by_label("Name").click_secondary();
    fixture.run();

    assert_eq!(fixture.selected_columns(), [0, 2].into_iter().collect());
}

#[test]
fn right_click_on_a_row_opens_the_row_menu_and_selects_it() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("3").click_secondary();
    fixture.run();
    fixture.run();

    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());
    fixture.get_by_label("Add Row Above");
    fixture.get_by_label("Delete Row(s)");
}

#[test]
fn a_row_menu_switches_away_from_a_column_selection() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.columns.insert(0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.get_by_label("3").click_secondary();
    fixture.run();
    fixture.run();

    assert_eq!(fixture.selected_rows(), [2].into_iter().collect());
    assert!(fixture.selected_columns().is_empty());
    fixture.get_by_label("Add Row Above");
}

#[test]
fn right_click_on_a_cell_opens_the_cell_menu() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Bob").click_secondary();
    fixture.run();
    fixture.run();

    assert_eq!(fixture.selected_rows(), [1].into_iter().collect());
    fixture.get_by_label("Copy");
    fixture.get_by_label("Paste");
    fixture.get_by_label("Select All");
}

#[test]
fn a_context_menu_owns_the_keyboard() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Bob").click_secondary();
    fixture.run();
    fixture.run();
    fixture.take_actions();

    fixture.key_press(Key::ArrowDown);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "arrow keys must stay with the menu"
    );
    fixture.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "Command+A must stay with the menu"
    );
    fixture.key_press(Key::Delete);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "Delete must stay with the menu"
    );

    assert_eq!(
        fixture
            .state
            .borrow()
            .focus
            .map(|focus| (focus.row, focus.column)),
        Some((1, 0)),
        "shortcuts must not move the grid focus behind an open menu"
    );
    assert_eq!(fixture.selected_rows(), [1].into_iter().collect());
}

#[test]
fn the_menu_renders_translated_labels() {
    let source = sample_table();
    let options = GridOptions {
        language: Language::Fr,
        ..GridOptions::default()
    };
    let mut fixture = grid_with_options(&source, options);
    fixture.run();
    fixture.get_by_label("Amount").click_secondary();
    fixture.run();
    fixture.run();

    fixture.get_by_label("Trier par ordre croissant");
}

#[test]
fn rename_starts_with_the_current_header_text() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Amount").click_secondary();
    fixture.run();
    fixture.run();
    fixture.get_by_label("Rename Column").click();
    fixture.run();
    fixture.run();

    let state = fixture.state.borrow();
    let editor = state
        .header_editing
        .as_ref()
        .expect("the header is editing");
    assert_eq!(editor.text, "Amount");
    assert_eq!(editor.column, source.column_ids()[1]);
}

#[test]
fn rename_commits_on_return_with_the_edited_text() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Amount").click_secondary();
    fixture.run();
    fixture.run();
    fixture.get_by_label("Rename Column").click();
    fixture.run();
    fixture.run();
    fixture.take_actions();

    fixture.get_by_role(Role::TextInput).type_text("!");
    fixture.run();
    fixture.key_press(Key::Enter);
    fixture.run();

    assert!(fixture.state.borrow().header_editing.is_none());
    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::RenameColumn {
            column: 1,
            name: "Amount!".to_owned(),
        }]
    );
}

#[test]
fn rename_cancels_on_escape() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Amount").click_secondary();
    fixture.run();
    fixture.run();
    fixture.get_by_label("Rename Column").click();
    fixture.run();
    fixture.run();
    fixture.take_actions();

    fixture.get_by_role(Role::TextInput).type_text("!");
    fixture.run();
    fixture.key_press(Key::Escape);
    fixture.run();

    assert!(fixture.state.borrow().header_editing.is_none());
    assert!(fixture.take_actions().is_empty());
}

#[test]
fn an_ime_composition_commit_does_not_commit_the_header() {
    let source = sample_table();
    let mut state = GridState::default();
    assert!(state.start_renaming(&source, 0));
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();

    fixture.event(Event::Ime(ImeEvent::Preedit {
        text: "に".to_owned(),
        active_range_chars: None,
    }));
    fixture.run();
    assert!(fixture.take_actions().is_empty());

    fixture.event(Event::Ime(ImeEvent::Commit("日本".to_owned())));
    fixture.key_press(Key::Enter);
    fixture.run();
    assert!(
        fixture.take_actions().is_empty(),
        "Enter during an IME commit must not commit the header"
    );
    let state = fixture.state.borrow();
    assert_eq!(
        state
            .header_editing
            .as_ref()
            .map(|editor| editor.text.as_str()),
        Some("Name日本")
    );
    drop(state);

    fixture.key_press(Key::Enter);
    fixture.run();
    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::RenameColumn {
            column: 0,
            name: "Name日本".to_owned(),
        }]
    );
}

#[test]
fn an_empty_column_name_is_a_valid_rename() {
    let source = CountingSource::with_headers(1, 1, Some(vec![String::new()]));
    let mut state = GridState::default();
    assert!(state.start_renaming(&source, 0));
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();

    fixture.key_press(Key::Enter);
    fixture.run();

    assert!(fixture.state.borrow().header_editing.is_none());
    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::RenameColumn {
            column: 0,
            name: String::new(),
        }]
    );
}

#[test]
fn renaming_tracks_the_column_identity_across_a_reorder() {
    let source = sample_table();
    let mut state = GridState::default();
    assert!(state.start_renaming(&source, 1));
    // Simulate the model moving another column in front while the editor is
    // open: the committed name must land on the same column.
    let mut presenter_ids = source.column_ids();
    presenter_ids.swap(0, 1);
    let reordered = ReorderedSource {
        source: &source,
        ids: presenter_ids,
    };
    let mut fixture = GridFixture::new(&reordered, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();

    fixture.key_press(Key::Enter);
    fixture.run();

    assert_eq!(
        fixture.take_actions(),
        vec![GridAction::RenameColumn {
            column: 0,
            name: "Amount".to_owned(),
        }]
    );
}

/// A source whose column identities are reordered while the cell data stays
/// put; the header editor must resolve its column by identity, not by index.
struct ReorderedSource<'a> {
    source: &'a dyn tabtool_core::RowSource,
    ids: Vec<tabtool_core::ColumnId>,
}

impl tabtool_core::RowSource for ReorderedSource<'_> {
    fn row_count(&self) -> usize {
        self.source.row_count()
    }

    fn column_count(&self) -> usize {
        self.source.column_count()
    }

    fn column_ids(&self) -> Vec<tabtool_core::ColumnId> {
        self.ids.clone()
    }

    fn headers(&self) -> Option<Vec<String>> {
        self.source.headers()
    }

    fn cell(&self, row: usize, column: usize) -> String {
        self.source.cell(row, column)
    }
}

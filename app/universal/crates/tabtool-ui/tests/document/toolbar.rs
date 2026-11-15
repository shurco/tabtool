//! Toolbar composition and enablement.

use tabtool_core::{CsvConfiguration, Encoding, FilterCriterion, FilterOperator, RowSource};
use tabtool_ui::grid::GridSelection;
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{Fixture, MemoryStorage, is_disabled, loaded, untitled};

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn rows(rows: impl IntoIterator<Item = usize>) -> GridSelection {
    GridSelection {
        rows: rows.into_iter().collect(),
        columns: Default::default(),
    }
}

fn columns(columns: impl IntoIterator<Item = usize>) -> GridSelection {
    GridSelection {
        rows: Default::default(),
        columns: columns.into_iter().collect(),
    }
}

#[test]
fn toolbar_shows_every_command_and_initial_enablement() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    for label in [
        "Add Row Above",
        "Add Row Below",
        "Add Column Left",
        "Add Column Right",
        "Delete Selection",
        "Export",
        "Show Filter",
    ] {
        assert!(
            fixture.query_by_label(label).is_some(),
            "missing toolbar command {label}"
        );
    }

    assert!(is_disabled(&fixture, "Add Row Above"));
    assert!(!is_disabled(&fixture, "Add Row Below"));
    assert!(is_disabled(&fixture, "Add Column Left"));
    assert!(!is_disabled(&fixture, "Add Column Right"));
    assert!(is_disabled(&fixture, "Delete Selection"));
    assert!(!is_disabled(&fixture, "Export"));
    assert!(!is_disabled(&fixture, "Show Filter"));
}

#[test]
fn toolbar_commands_act_on_the_grid_selection() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    fixture.set_selection(rows([0]));
    fixture.run();
    assert!(!is_disabled(&fixture, "Add Row Above"));
    assert!(!is_disabled(&fixture, "Delete Selection"));

    fixture.get_by_label("Add Row Above").click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("source").row_count()),
        3
    );
    assert_eq!(fixture.grid().selection.rows, [0].into_iter().collect());

    fixture.get_by_label("Delete Selection").click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("source").row_count()),
        2
    );
    assert!(fixture.grid().selection.is_empty());
}

#[test]
fn column_commands_need_a_column_selection_and_keep_it() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();
    fixture.set_selection(columns([1]));
    fixture.run();

    assert!(!is_disabled(&fixture, "Add Column Left"));
    assert!(!is_disabled(&fixture, "Delete Selection"));

    fixture.get_by_label("Add Column Left").click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("source").column_count()),
        3
    );
    assert_eq!(fixture.grid().selection.columns, [1].into_iter().collect());
}

#[test]
fn deleting_the_last_column_is_disabled() {
    let storage = MemoryStorage::new(b"a\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a\n"),
        Default::default(),
    );
    fixture.run();
    fixture.set_selection(columns([0]));
    fixture.run();

    assert!(is_disabled(&fixture, "Delete Selection"));
}

#[test]
fn the_filter_button_toggles_the_bar_and_its_label() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut fixture = Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"a,b\n1,2\n"),
        Default::default(),
    );
    fixture.run();

    fixture.get_by_label("Show Filter").click();
    fixture.run();
    assert!(
        fixture.with_document(|document| document.row_filter().is_bar_visible),
        "the filter bar must open"
    );
    assert!(fixture.query_by_label("Hide Filter").is_some());

    fixture.get_by_label("Hide Filter").click();
    fixture.run();
    assert!(
        !fixture.with_document(|document| document.row_filter().is_bar_visible),
        "the filter bar must close"
    );
}

#[test]
fn a_filtered_view_disables_row_structure_but_keeps_columns() {
    let storage = MemoryStorage::new(b"a,1\nb,2\na,3\n");
    let mut document = loaded(&storage, configuration());
    let column_id = document.table().expect("table").column_ids()[0];
    document.row_filter_mut().criteria = vec![FilterCriterion {
        id: 1,
        is_enabled: true,
        column_id: Some(column_id),
        op: FilterOperator::Equals,
        value: "a".to_owned(),
    }];
    document.apply_all_filters();
    document.selection_mut().rows.insert(0);

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();
    fixture.set_selection(rows([1]));
    fixture.run();

    assert!(is_disabled(&fixture, "Add Row Above"));
    assert!(is_disabled(&fixture, "Add Row Below"));
    assert!(is_disabled(&fixture, "Delete Selection"));
    assert!(!is_disabled(&fixture, "Add Column Right"));
}

#[test]
fn large_mode_disables_structure_export_and_filter() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
    let mut document = crate::support::large_ready(&path);
    document.selection_mut().rows.insert(0);
    document.selection_mut().columns.insert(0);

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    for label in [
        "Add Row Above",
        "Add Row Below",
        "Add Column Left",
        "Add Column Right",
        "Delete Selection",
        "Export",
        "Show Filter",
    ] {
        assert!(is_disabled(&fixture, label), "{label} must be disabled");
    }
}

#[test]
fn untitled_documents_export_and_filter() {
    let storage = MemoryStorage::new(b"");
    let mut fixture = Fixture::new(
        untitled(CsvConfiguration {
            encoding: Encoding::Utf8,
            ..configuration()
        }),
        storage,
        Default::default(),
    );
    fixture.run();

    assert!(!is_disabled(&fixture, "Export"));
    assert!(!is_disabled(&fixture, "Show Filter"));
}

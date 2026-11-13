//! The filter bar: draft vs applied criteria, actions and operator labels.

#[path = "document/support.rs"]
mod support;

use tabtool_core::{CsvConfiguration, FilterOperator};
use tabtool_i18n::{Language, tr, tr_in};
use tabtool_ui::filter::operator_title;
use tabtool_ui::harness::kittest::{NodeT, Queryable};

use support::{Fixture, MemoryStorage, loaded};

const CSV: &[u8] = b"name,kind\napple,fruit\ncarrot,veg\napricot,fruit\n";

fn configuration() -> CsvConfiguration {
    CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    }
}

fn fixture() -> Fixture {
    let storage = MemoryStorage::new(CSV);
    Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(CSV),
        Default::default(),
    )
}

fn open(mut fixture: Fixture) -> Fixture {
    fixture.run();
    fixture.get_by_label("Show Filter").click();
    fixture.run();
    fixture
}

fn type_value(fixture: &mut Fixture, index: usize, text: &str) {
    let _ = fixture.query_all_by_label("Value").nth(index).map(|node| {
        node.focus();
    });
    fixture.run();
    let node = fixture
        .query_all_by_label("Value")
        .nth(index)
        .expect("the criterion has a value field");
    node.type_text(text);
    fixture.run();
}

fn visible_rows(fixture: &Fixture) -> Option<Vec<usize>> {
    fixture.with_document(|document| document.row_filter().visible_rows.clone())
}

fn choose_column(fixture: &mut Fixture, index: usize, name: &str) {
    fixture
        .query_all_by_label("Column")
        .nth(index)
        .expect("the criterion has a column picker")
        .click();
    fixture.run();
    fixture
        .get_by_role_and_label(egui::accesskit::Role::Button, name)
        .click_accesskit();
    fixture.run();
}

fn choose_operator(fixture: &mut Fixture, index: usize, title: &str) {
    fixture
        .query_all_by_label("Operator")
        .nth(index)
        .expect("the criterion has an operator picker")
        .click();
    fixture.run();
    fixture
        .get_by_role_and_label(egui::accesskit::Role::Button, title)
        .click_accesskit();
    fixture.run();
}

#[test]
fn operators_follow_the_document_order_and_the_catalog() {
    let titles: Vec<&str> = FilterOperator::ALL
        .iter()
        .map(|operator| operator_title(Language::En, *operator))
        .collect();
    assert_eq!(
        titles,
        [
            "=",
            "≠",
            "Contains",
            "Not contains",
            "Begins with",
            "Ends with",
            ">",
            "<",
            "≥",
            "≤",
            "Is empty",
            "Is not empty",
            "Matches regex",
        ]
    );
    assert_eq!(
        operator_title(Language::De, FilterOperator::Contains),
        "Enthält"
    );

    for key in [
        "Contains",
        "Not contains",
        "Begins with",
        "Ends with",
        "Is empty",
        "Is not empty",
        "Matches regex",
    ] {
        assert_eq!(tr(Language::En, key), key, "the English key is the label");
        for language in Language::ALL.into_iter().skip(1) {
            assert!(
                tr_in(language, key).is_some(),
                "{key} is missing from {:?}",
                language
            );
        }
    }
}

#[test]
fn the_bar_edits_a_draft_and_rows_change_only_on_apply() {
    let mut fixture = open(fixture());
    assert!(fixture.query_by_label("Apply All").is_some());
    assert!(fixture.query_by_label("Enable Criterion").is_some());
    assert!(fixture.query_by_label("Column").is_some());
    assert!(fixture.query_by_label("Operator").is_some());
    assert!(fixture.query_by_label("Value").is_some());

    type_value(&mut fixture, 0, "ap");
    assert_eq!(visible_rows(&fixture), None, "typing must not filter");

    fixture.get_by_label("Apply").click();
    fixture.run();
    assert_eq!(visible_rows(&fixture), Some(vec![0, 2]));

    fixture.run();
    assert!(
        fixture.query_by_label("Hide Filter (1)").is_some(),
        "the toolbar reports the applied-criterion count"
    );
}

#[test]
fn apply_all_combines_every_criterion_and_clear_resets_the_bar() {
    let mut fixture = open(fixture());
    fixture.get_by_label("Add Criterion").click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria.len()),
        2
    );

    choose_column(&mut fixture, 1, "kind");
    type_value(&mut fixture, 0, "ap");
    type_value(&mut fixture, 1, "fruit");

    fixture.get_by_label("Apply All").click();
    fixture.run();
    assert_eq!(visible_rows(&fixture), Some(vec![0, 2]));

    fixture.get_by_label("Clear").click();
    fixture.run();
    assert_eq!(visible_rows(&fixture), None);
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria.len()),
        1,
        "Clear returns the bar to one empty row"
    );
}

#[test]
fn the_empty_operator_disables_the_value_field() {
    let storage = MemoryStorage::new(b"name,kind\napple,\ncarrot,veg\n");
    let mut fixture = open(Fixture::new(
        loaded(&storage, configuration()),
        MemoryStorage::new(b"name,kind\napple,\ncarrot,veg\n"),
        Default::default(),
    ));
    choose_column(&mut fixture, 0, "kind");
    choose_operator(&mut fixture, 0, "Is empty");
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].op),
        FilterOperator::IsEmpty
    );

    assert!(
        fixture.get_by_label("Value").accesskit_node().is_disabled(),
        "the emptiness operators ignore the value"
    );

    fixture.get_by_label("Apply All").click();
    fixture.run();
    assert_eq!(visible_rows(&fixture), Some(vec![0]));
}

#[test]
fn an_invalid_regex_is_flagged_and_never_filters() {
    let mut fixture = open(fixture());
    choose_operator(&mut fixture, 0, "Matches regex");
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].op),
        FilterOperator::MatchesRegex
    );
    type_value(&mut fixture, 0, "[");
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].value.clone()),
        "[",
        "the pattern is in the draft"
    );

    assert!(
        fixture
            .query_by_label("Invalid regular expression")
            .is_some(),
        "an invalid pattern must be explained"
    );

    fixture.get_by_label("Apply All").click();
    fixture.run();
    assert_eq!(visible_rows(&fixture), None);
    assert_eq!(
        fixture.with_document(|document| document.row_filter().applied.clone()),
        None
    );
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].value.clone()),
        "[",
        "the draft keeps the text"
    );
}

#[test]
fn add_and_remove_keep_at_least_one_row() {
    let mut fixture = open(fixture());
    fixture.get_by_label("Add Criterion").click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria.len()),
        2
    );

    fixture
        .query_all_by_label("Remove Criterion")
        .next()
        .expect("the criterion can be removed")
        .click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria.len()),
        1
    );

    fixture
        .query_all_by_label("Remove Criterion")
        .next()
        .expect("the last criterion can be reset")
        .click();
    fixture.run();
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria.len()),
        1,
        "the last row is reset instead of removed"
    );
}

#[test]
fn hiding_the_bar_keeps_the_draft_and_the_applied_filter() {
    let mut fixture = open(fixture());
    type_value(&mut fixture, 0, "ap");
    fixture.get_by_label("Apply").click();
    fixture.run();

    fixture.get_by_label("Hide").click();
    fixture.run();
    assert!(fixture.query_by_label("Apply All").is_none());
    assert!(!fixture.with_document(|document| document.row_filter().is_bar_visible));
    assert_eq!(visible_rows(&fixture), Some(vec![0, 2]));
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].value.clone()),
        "ap"
    );

    fixture.get_by_label("Show Filter (1)").click();
    fixture.run();
    assert!(fixture.query_by_label("Apply All").is_some());
    assert_eq!(
        fixture.with_document(|document| document.row_filter().criteria[0].value.clone()),
        "ap"
    );
}

#[test]
fn applying_a_filter_commits_the_open_cell_editor_first() {
    let mut fixture = open(fixture());
    type_value(&mut fixture, 0, "z");

    fixture.get_by_label("apple").click();
    fixture.run();
    fixture.get_by_label("apple").click();
    fixture.run();
    fixture.run();

    fixture
        .get_by_role_and_label(egui::accesskit::Role::TextInput, "name")
        .type_text("z");
    fixture.run();
    assert!(
        fixture.with_document(|document| document.row_filter().visible_rows.is_none()),
        "typing in the cell must not filter"
    );

    fixture.get_by_label("Apply").click();
    fixture.run();

    assert_eq!(
        fixture.with_document(|document| document.row_source().expect("the table").cell(0, 0)),
        "applez",
        "the editor commits before the filter hides rows"
    );
    assert_eq!(visible_rows(&fixture), Some(vec![0]));
}

#[test]
fn a_large_document_never_shows_the_filter_bar() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, CSV).expect("the fixture is written");
    let mut document = support::large_ready(&path);
    document.row_filter_mut().is_bar_visible = true;

    let mut fixture = Fixture::new(document, MemoryStorage::new(b""), Default::default());
    fixture.run();

    assert!(
        fixture.query_by_label("Apply All").is_none(),
        "large mode hides the filter bar even when the flag is set"
    );
}

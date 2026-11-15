//! Port of `RowFilterModelTests.swift`.
//!
//! The model keeps the draft criteria edited by the filter bar separate from
//! the applied filter. `visible_rows` is a snapshot evaluated by an apply or a
//! refresh, never by a cell edit. The refresh remaps criteria to new columns at
//! the same positions after a reload and drops criteria whose column is gone.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use tabtool_core::{ColumnId, CriterionId, CsvTable, FilterCriterion, FilterOperator, RowFilter};
use tabtool_document::RowFilterModel;

fn table() -> CsvTable {
    CsvTable::from_rows(
        vec![
            vec!["a".to_owned(), "1".to_owned()],
            vec!["b".to_owned(), "2".to_owned()],
            vec!["a".to_owned(), "3".to_owned()],
            vec!["c".to_owned(), "4".to_owned()],
        ],
        Some(vec!["name".to_owned(), "number".to_owned()]),
        2,
    )
}

fn name_id(table: &CsvTable) -> ColumnId {
    table.columns[0].id
}

fn number_id(table: &CsvTable) -> ColumnId {
    table.columns[1].id
}

static NEXT_ID: AtomicU64 = AtomicU64::new(10_000);

fn criterion(column: Option<ColumnId>, op: FilterOperator, value: &str) -> FilterCriterion {
    FilterCriterion {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        is_enabled: true,
        column_id: column,
        op,
        value: value.to_owned(),
    }
}

fn model(criteria: Vec<FilterCriterion>) -> RowFilterModel {
    let mut model = RowFilterModel::default();
    model.criteria = criteria;
    model
}

fn ids(criteria: &[FilterCriterion]) -> Vec<Option<ColumnId>> {
    criteria
        .iter()
        .map(|criterion| criterion.column_id)
        .collect()
}

#[test]
fn starts_with_one_empty_criterion_and_no_filter() {
    let model = RowFilterModel::default();
    assert_eq!(model.criteria.len(), 1);
    assert_eq!(model.criteria[0].column_id, None);
    assert!(model.criteria[0].value.is_empty());
    assert_eq!(model.applied, None);
    assert_eq!(model.visible_rows, None);
    assert!(!model.is_filtering());
    assert!(!model.is_bar_visible);
}

#[test]
fn apply_all_combines_every_criterion_with_and() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(name_id(&table)), FilterOperator::Equals, "a"),
        criterion(Some(number_id(&table)), FilterOperator::GreaterThan, "1"),
    ]);
    model.apply_all(&table, ".");

    assert_eq!(model.visible_rows, Some(vec![2]));
    assert!(model.is_filtering());
    assert_eq!(
        model.applied,
        Some(RowFilter {
            criteria: model.criteria.clone(),
        })
    );
}

#[test]
fn apply_criterion_uses_only_that_criterion_even_when_disabled() {
    let table = table();
    let mut first = criterion(Some(name_id(&table)), FilterOperator::Equals, "a");
    first.is_enabled = false;
    let first_id = first.id;
    let second = criterion(Some(number_id(&table)), FilterOperator::GreaterThan, "1");
    let second_id = second.id;
    let mut model = model(vec![first, second]);

    model.apply(first_id, &table, ".");
    assert_eq!(model.visible_rows, Some(vec![0, 2]));
    assert_eq!(model.applied.as_ref().expect("applied").criteria.len(), 1);

    model.apply(second_id, &table, ".");
    assert_eq!(model.visible_rows, Some(vec![1, 2, 3]));
}

#[test]
fn apply_unknown_criterion_changes_nothing() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Equals,
        "a",
    )]);
    model.apply(CriterionId::MAX, &table, ".");
    assert!(!model.is_filtering());
}

#[test]
fn apply_honours_the_decimal_mark() {
    let source = CsvTable::from_rows(
        vec![vec!["1,5".to_owned()], vec!["2,5".to_owned()]],
        None,
        1,
    );
    let mut model = model(vec![criterion(
        Some(source.columns[0].id),
        FilterOperator::GreaterThan,
        "2,0",
    )]);
    model.apply_all(&source, ",");
    assert_eq!(model.visible_rows, Some(vec![1]));
}

#[test]
fn clear_removes_the_filter_and_resets_the_draft() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(name_id(&table)), FilterOperator::Contains, "a"),
        criterion(Some(number_id(&table)), FilterOperator::Contains, "1"),
    ]);
    model.apply_all(&table, ".");
    model.clear();

    assert_eq!(model.applied, None);
    assert_eq!(model.visible_rows, None);
    assert_eq!(model.criteria.len(), 1);
    assert_eq!(model.criteria[0].column_id, None);
    assert!(model.criteria[0].value.is_empty());
}

#[test]
fn add_criterion_inserts_an_empty_row_after_the_given_one() {
    let table = table();
    let first = criterion(Some(name_id(&table)), FilterOperator::Contains, "a");
    let first_id = first.id;
    let second = criterion(Some(number_id(&table)), FilterOperator::Contains, "1");
    let second_id = second.id;
    let mut model = model(vec![first, second]);

    model.add_criterion(first_id);
    assert_eq!(model.criteria.len(), 3);
    assert_eq!(model.criteria[0].id, first_id);
    assert_eq!(model.criteria[1].column_id, None);
    assert_eq!(model.criteria[2].id, second_id);
}

#[test]
fn add_criterion_after_an_unknown_id_appends() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Contains,
        "a",
    )]);
    model.add_criterion(CriterionId::MAX);
    assert_eq!(model.criteria.len(), 2);
    assert_eq!(model.criteria[1].column_id, None);
}

#[test]
fn remove_criterion_drops_it_but_always_keeps_one_row() {
    let table = table();
    let first = criterion(Some(name_id(&table)), FilterOperator::Contains, "a");
    let first_id = first.id;
    let second = criterion(Some(number_id(&table)), FilterOperator::Contains, "1");
    let second_id = second.id;
    let mut model = model(vec![first, second]);

    model.remove_criterion(first_id);
    assert_eq!(
        model
            .criteria
            .iter()
            .map(|criterion| criterion.id)
            .collect::<Vec<_>>(),
        vec![second_id]
    );

    model.remove_criterion(second_id);
    assert_eq!(model.criteria.len(), 1);
    assert_ne!(model.criteria[0].id, second_id);
    assert_eq!(model.criteria[0].column_id, None);
    assert!(model.criteria[0].value.is_empty());
}

#[test]
fn remove_criterion_does_not_touch_the_applied_filter() {
    let table = table();
    let first = criterion(Some(name_id(&table)), FilterOperator::Contains, "a");
    let first_id = first.id;
    let second = criterion(Some(number_id(&table)), FilterOperator::Contains, "1");
    let mut model = model(vec![first, second]);
    model.apply_all(&table, ".");

    model.remove_criterion(first_id);
    assert_eq!(model.applied.as_ref().expect("applied").criteria.len(), 2);
}

#[test]
fn refresh_reevaluates_the_applied_filter() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Equals,
        "a",
    )]);
    model.apply_all(&table, ".");
    assert_eq!(model.visible_rows, Some(vec![0, 2]));

    let mut edited = table.clone();
    edited.set_cell("z".to_owned(), 0, 0);
    model.refresh(&edited, ".");
    assert_eq!(model.visible_rows, Some(vec![2]));
}

#[test]
fn refresh_without_a_filter_keeps_everything_visible() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Contains,
        "a",
    )]);
    model.refresh(&table, ".");
    assert_eq!(model.visible_rows, None);
    assert!(!model.is_filtering());
}

#[test]
fn refresh_drops_applied_criteria_of_removed_columns() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(name_id(&table)), FilterOperator::Equals, "a"),
        criterion(Some(number_id(&table)), FilterOperator::GreaterThan, "1"),
    ]);
    model.apply_all(&table, ".");

    let mut without_number = table.clone();
    without_number.delete_columns(&BTreeSet::from([1]));
    model.refresh(&without_number, ".");

    assert_eq!(
        ids(&model.applied.as_ref().expect("applied").criteria),
        vec![Some(name_id(&table))]
    );
    assert_eq!(model.visible_rows, Some(vec![0, 2]));
}

#[test]
fn refresh_clears_the_filter_when_no_criterion_keeps_its_column() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(number_id(&table)),
        FilterOperator::GreaterThan,
        "1",
    )]);
    model.apply_all(&table, ".");

    let mut without_number = table.clone();
    without_number.delete_columns(&BTreeSet::from([1]));
    model.refresh(&without_number, ".");

    assert!(!model.is_filtering());
    assert_eq!(model.visible_rows, None);
}

#[test]
fn refresh_detaches_draft_criteria_from_removed_columns() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(name_id(&table)), FilterOperator::Contains, "a"),
        criterion(Some(number_id(&table)), FilterOperator::Contains, "1"),
    ]);
    let mut without_number = table.clone();
    without_number.delete_columns(&BTreeSet::from([1]));

    model.refresh(&without_number, ".");
    assert_eq!(model.criteria.len(), 2);
    assert_eq!(model.criteria[0].column_id, Some(name_id(&table)));
    assert_eq!(model.criteria[1].column_id, None);
}

#[test]
fn refresh_keeps_criteria_on_the_same_positions_after_a_reload() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(name_id(&table)), FilterOperator::Equals, "a"),
        criterion(Some(number_id(&table)), FilterOperator::GreaterThan, "1"),
    ]);
    model.apply_all(&table, ".");

    let reloaded = CsvTable::from_rows(
        vec![
            vec!["a".to_owned(), "0".to_owned()],
            vec!["a".to_owned(), "5".to_owned()],
            vec!["b".to_owned(), "9".to_owned()],
        ],
        Some(vec!["name".to_owned(), "number".to_owned()]),
        2,
    );
    model.refresh(&reloaded, ".");
    let reloaded_ids: Vec<ColumnId> = reloaded.columns.iter().map(|column| column.id).collect();

    assert_eq!(
        ids(&model.criteria),
        reloaded_ids.iter().copied().map(Some).collect::<Vec<_>>()
    );
    assert_eq!(
        ids(&model.applied.as_ref().expect("applied").criteria),
        reloaded_ids.iter().copied().map(Some).collect::<Vec<_>>()
    );
    assert_eq!(model.visible_rows, Some(vec![1]));
}

#[test]
fn refresh_keeps_remapped_criteria_on_the_next_edit() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Equals,
        "a",
    )]);
    model.apply_all(&table, ".");

    let reloaded = CsvTable::from_rows(vec![vec!["a".to_owned(), "0".to_owned()]], None, 2);
    model.refresh(&reloaded, ".");

    let mut edited = reloaded.clone();
    edited.set_cell("z".to_owned(), 0, 0);
    model.refresh(&edited, ".");
    assert!(model.is_filtering());
    assert_eq!(model.visible_rows, Some(vec![]));
}

#[test]
fn fill_missing_columns_only_touches_criteria_without_a_column() {
    let table = table();
    let mut model = model(vec![
        criterion(Some(number_id(&table)), FilterOperator::Contains, ""),
        criterion(None, FilterOperator::Contains, ""),
    ]);
    model.fill_missing_columns(name_id(&table));
    assert_eq!(
        ids(&model.criteria),
        vec![Some(number_id(&table)), Some(name_id(&table))]
    );
}

#[test]
fn apply_without_an_active_criterion_leaves_no_filter() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Equals,
        "",
    )]);
    model.apply_all(&table, ".");
    assert!(!model.is_filtering());
    assert_eq!(model.visible_rows, None);

    let id = model.criteria[0].id;
    model.apply(id, &table, ".");
    assert!(!model.is_filtering());
    assert_eq!(model.visible_rows, None);
}

#[test]
fn applying_only_inactive_criteria_clears_an_applied_filter() {
    let table = table();
    let mut model = model(vec![criterion(
        Some(name_id(&table)),
        FilterOperator::Equals,
        "a",
    )]);
    model.apply_all(&table, ".");
    assert_eq!(model.visible_rows, Some(vec![0, 2]));

    model.criteria[0].value.clear();
    model.apply_all(&table, ".");
    assert!(!model.is_filtering());
    assert_eq!(model.visible_rows, None);
    assert_eq!(model.criteria[0].column_id, Some(name_id(&table)));
}

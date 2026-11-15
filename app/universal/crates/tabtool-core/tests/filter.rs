//! Ports of the Apple `RowFilterTests` suite, plus the Review Focus cases for
//! skipped criteria, invalid regexes, emptiness and numeric comparison.

use tabtool_core::{CsvTable, FilterCriterion, FilterOperator, Locale, RowFilter};

const NAMES: [&str; 6] = ["Alice", "bob", "Carol", "alice smith", "", "Dave"];
const VALUES: [&str; 6] = ["10", "9", "2.5", "abc", "", "10.0"];

fn locale() -> Locale {
    "en-US".parse().expect("test locale is valid")
}

fn make_table() -> CsvTable {
    let rows = NAMES
        .iter()
        .zip(VALUES)
        .map(|(name, value)| vec![(*name).to_owned(), value.to_owned()])
        .collect();
    CsvTable::from_rows(rows, None, 2)
}

fn criterion(column_id: Option<u64>, op: FilterOperator, value: &str) -> FilterCriterion {
    FilterCriterion {
        id: 1,
        is_enabled: true,
        column_id,
        op,
        value: value.to_owned(),
    }
}

fn single(table: &CsvTable, column: usize, op: FilterOperator, value: &str) -> Vec<usize> {
    single_with_mark(table, column, op, value, ".")
}

fn single_with_mark(
    table: &CsvTable,
    column: usize,
    op: FilterOperator,
    value: &str,
    decimal_mark: &str,
) -> Vec<usize> {
    let criterion = criterion(Some(table.columns[column].id), op, value);
    let filter = RowFilter {
        criteria: vec![criterion],
    };
    filter.matching_rows(table, decimal_mark, &locale())
}

/// Rows of [`make_table`] matched by one criterion on `column`.
fn matched(op: FilterOperator, value: &str, column: usize) -> Vec<usize> {
    single(&make_table(), column, op, value)
}

#[test]
fn equals_is_case_insensitive_text() {
    assert_eq!(matched(FilterOperator::Equals, "ALICE", 0), vec![0]);
}

#[test]
fn not_equals_keeps_everything_else() {
    assert_eq!(
        matched(FilterOperator::NotEquals, "alice", 0),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn contains_is_case_insensitive() {
    assert_eq!(matched(FilterOperator::Contains, "ALIC", 0), vec![0, 3]);
}

#[test]
fn not_contains_excludes_matches() {
    assert_eq!(
        matched(FilterOperator::NotContains, "alic", 0),
        vec![1, 2, 4, 5]
    );
}

#[test]
fn begins_with_anchors_at_start() {
    assert_eq!(matched(FilterOperator::BeginsWith, "ALICE", 0), vec![0, 3]);
    assert!(matched(FilterOperator::BeginsWith, "smith", 0).is_empty());
}

#[test]
fn ends_with_anchors_at_end() {
    assert_eq!(matched(FilterOperator::EndsWith, "SMITH", 0), vec![3]);
    assert_eq!(matched(FilterOperator::EndsWith, "alice", 0), vec![0]);
}

#[test]
fn numeric_equals_compares_numbers() {
    assert_eq!(matched(FilterOperator::Equals, "10", 1), vec![0, 5]);
}

#[test]
fn numeric_ordering_compares_numbers_not_text() {
    // Text order would put "10" before "9"; the non-numeric "abc" falls back to text.
    assert_eq!(matched(FilterOperator::GreaterThan, "9", 1), vec![0, 3, 5]);
    assert_eq!(matched(FilterOperator::LessThan, "10", 1), vec![1, 2]);
    assert_eq!(
        matched(FilterOperator::GreaterOrEqual, "9", 1),
        vec![0, 1, 3, 5]
    );
    assert_eq!(matched(FilterOperator::LessOrEqual, "9", 1), vec![1, 2]);
}

#[test]
fn non_numeric_side_falls_back_to_case_insensitive_text_comparison() {
    assert_eq!(matched(FilterOperator::Equals, "ABC", 1), vec![3]);
    assert_eq!(
        matched(FilterOperator::GreaterThan, "ALICE Z", 0),
        vec![1, 2, 5]
    );
    assert_eq!(matched(FilterOperator::LessThan, "b", 0), vec![0, 3]);
}

#[test]
fn empty_cell_never_satisfies_equals_or_ordering() {
    assert!(!matched(FilterOperator::Equals, "x", 0).contains(&4));
    assert!(!matched(FilterOperator::LessThan, "z", 0).contains(&4));
    assert!(!matched(FilterOperator::LessOrEqual, "10", 1).contains(&4));
    assert!(matched(FilterOperator::NotEquals, "x", 0).contains(&4));
}

#[test]
fn text_operators_treat_composed_and_decomposed_forms_as_equal() {
    let table = CsvTable::from_rows(
        vec![
            vec!["éclair".to_owned()],
            vec!["e\u{301}clair".to_owned()],
            vec!["other".to_owned()],
        ],
        None,
        1,
    );

    assert_eq!(single(&table, 0, FilterOperator::Contains, "é"), vec![0, 1]);
    assert_eq!(
        single(&table, 0, FilterOperator::NotContains, "e\u{301}"),
        vec![2]
    );
    assert_eq!(
        single(&table, 0, FilterOperator::BeginsWith, "e\u{301}"),
        vec![0, 1]
    );
    assert_eq!(
        single(&table, 0, FilterOperator::EndsWith, "éclair"),
        vec![0, 1]
    );
    assert_eq!(
        single(&table, 0, FilterOperator::Equals, "e\u{301}clair"),
        vec![0, 1]
    );
    assert_eq!(
        single(&table, 0, FilterOperator::NotEquals, "éclair"),
        vec![2]
    );
}

#[test]
fn decimal_mark_controls_numeric_parsing() {
    let table = CsvTable::from_rows(
        vec![
            vec!["1,5".to_owned()],
            vec!["1,50".to_owned()],
            vec!["2,0".to_owned()],
        ],
        None,
        1,
    );
    assert_eq!(
        single_with_mark(&table, 0, FilterOperator::Equals, "1,5", ","),
        vec![0, 1]
    );
    assert_eq!(
        single_with_mark(&table, 0, FilterOperator::GreaterThan, "1,5", ","),
        vec![2]
    );
}

#[test]
fn is_empty_and_is_not_empty_ignore_the_value() {
    assert_eq!(matched(FilterOperator::IsEmpty, "ignored", 0), vec![4]);
    assert_eq!(
        matched(FilterOperator::IsNotEmpty, "", 0),
        vec![0, 1, 2, 3, 5]
    );
}

#[test]
fn whitespace_only_cell_counts_as_empty() {
    let table = CsvTable::from_rows(vec![vec![" ".to_owned()], vec!["x".to_owned()]], None, 1);
    assert_eq!(single(&table, 0, FilterOperator::IsEmpty, ""), vec![0]);
}

#[test]
fn regex_matches_anywhere_and_is_case_insensitive() {
    assert_eq!(matched(FilterOperator::MatchesRegex, "^a.*e$", 0), vec![0]);
    assert_eq!(
        matched(FilterOperator::MatchesRegex, "SMITH|dave", 0),
        vec![3, 5]
    );
}

#[test]
fn invalid_regex_is_invalid_and_ignored() {
    let table = make_table();
    let criterion = criterion(Some(table.columns[0].id), FilterOperator::MatchesRegex, "(");
    assert!(!criterion.is_valid());
    assert_eq!(
        single(&table, 0, FilterOperator::MatchesRegex, "(").len(),
        NAMES.len()
    );
}

#[test]
fn valid_criteria_report_valid() {
    assert!(criterion(None, FilterOperator::MatchesRegex, "a+").is_valid());
    assert!(criterion(None, FilterOperator::MatchesRegex, "").is_valid());
    assert!(criterion(None, FilterOperator::Contains, "(").is_valid());
}

#[test]
fn empty_value_is_ignored_for_value_operators() {
    for op in FilterOperator::ALL {
        if op.needs_value() {
            assert_eq!(matched(op, "", 0).len(), NAMES.len(), "{op:?}");
        }
    }
}

#[test]
fn needs_value_is_false_only_for_emptiness_operators() {
    let without_value: Vec<_> = FilterOperator::ALL
        .into_iter()
        .filter(|op| !op.needs_value())
        .collect();
    assert_eq!(
        without_value,
        vec![FilterOperator::IsEmpty, FilterOperator::IsNotEmpty]
    );
}

#[test]
fn disabled_criterion_is_ignored() {
    let table = make_table();
    let mut criterion = criterion(Some(table.columns[0].id), FilterOperator::Equals, "bob");
    criterion.is_enabled = false;
    let filter = RowFilter {
        criteria: vec![criterion],
    };
    assert_eq!(
        filter.matching_rows(&table, ".", &locale()).len(),
        NAMES.len()
    );
}

#[test]
fn criteria_combine_with_and() {
    let table = make_table();
    let filter = RowFilter {
        criteria: vec![
            criterion(Some(table.columns[0].id), FilterOperator::Contains, "alice"),
            criterion(Some(table.columns[1].id), FilterOperator::LessThan, "10"),
        ],
    };
    assert_eq!(
        filter.matching_rows(&table, ".", &locale()),
        Vec::<usize>::new()
    );

    let loose = RowFilter {
        criteria: vec![
            criterion(Some(table.columns[0].id), FilterOperator::Contains, "alice"),
            criterion(
                Some(table.columns[1].id),
                FilterOperator::GreaterOrEqual,
                "10",
            ),
        ],
    };
    // "abc" is not numeric, so it compares as text and sorts after "10".
    assert_eq!(loose.matching_rows(&table, ".", &locale()), vec![0, 3]);
}

#[test]
fn missing_or_unknown_column_is_ignored() {
    let table = make_table();
    let filter = RowFilter {
        criteria: vec![
            criterion(None, FilterOperator::Equals, "x"),
            criterion(Some(u64::MAX), FilterOperator::Equals, "x"),
        ],
    };
    assert_eq!(
        filter.matching_rows(&table, ".", &locale()).len(),
        NAMES.len()
    );
}

#[test]
fn no_active_criteria_returns_all_rows_in_order() {
    assert_eq!(
        RowFilter { criteria: vec![] }.matching_rows(&make_table(), ".", &locale()),
        (0..NAMES.len()).collect::<Vec<_>>()
    );
}

#[test]
fn empty_table_yields_no_rows() {
    let table = CsvTable::from_rows(Vec::new(), None, 0);
    let criterion = criterion(Some(table.columns[0].id), FilterOperator::IsEmpty, "");
    let filter = RowFilter {
        criteria: vec![criterion],
    };
    assert!(filter.matching_rows(&table, ".", &locale()).is_empty());
}

#[test]
fn is_active_needs_at_least_one_usable_criterion() {
    let table = make_table();
    let id = table.columns[0].id;
    let active = |criteria: Vec<FilterCriterion>| RowFilter { criteria }.is_active(&table, ".");

    assert!(!RowFilter { criteria: vec![] }.is_active(&table, "."));
    assert!(!active(vec![criterion(
        Some(id),
        FilterOperator::Contains,
        ""
    )]));
    assert!(!active(vec![{
        let mut disabled = criterion(Some(id), FilterOperator::Contains, "a");
        disabled.is_enabled = false;
        disabled
    }]));
    assert!(!active(vec![criterion(
        None,
        FilterOperator::Contains,
        "a"
    )]));
    assert!(!active(vec![criterion(
        Some(u64::MAX),
        FilterOperator::Contains,
        "a"
    )]));
    assert!(!active(vec![criterion(
        Some(id),
        FilterOperator::MatchesRegex,
        "("
    )]));
    assert!(active(vec![criterion(
        Some(id),
        FilterOperator::Contains,
        "a"
    )]));
    assert!(active(vec![criterion(
        Some(id),
        FilterOperator::IsEmpty,
        ""
    )]));
    assert!(active(vec![
        criterion(Some(id), FilterOperator::Contains, ""),
        criterion(Some(id), FilterOperator::Contains, "a"),
    ]));
}

#[test]
fn empty_regex_is_valid_but_never_active() {
    let table = make_table();
    let criterion = criterion(Some(table.columns[0].id), FilterOperator::MatchesRegex, "");
    assert!(criterion.is_valid());
    let filter = RowFilter {
        criteria: vec![criterion],
    };
    assert!(!filter.is_active(&table, "."));
    assert_eq!(
        filter.matching_rows(&table, ".", &locale()),
        (0..NAMES.len()).collect::<Vec<_>>()
    );
}

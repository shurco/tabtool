//! Port of `FormatChangePlannerTests.swift`.
//!
//! The planner maps a configuration edit to the smallest operation: an
//! in-memory decimal or header transform, or a disk re-read.

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::{FormatChange, plan_format_change};

fn changed(apply: impl FnOnce(&mut CsvConfiguration)) -> CsvConfiguration {
    let mut configuration = CsvConfiguration::default();
    apply(&mut configuration);
    configuration
}

#[test]
fn only_decimal_changes_plan_decimal() {
    let new = changed(|configuration| configuration.decimal_mark = ",".to_string());

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::DecimalMark(",".to_string())
    );
}

#[test]
fn only_header_changes_plan_header() {
    let new = changed(|configuration| configuration.first_row_as_header = true);

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::Header(true)
    );
}

#[test]
fn separator_change_plans_reparse() {
    let new = changed(|configuration| configuration.column_separator = ";".to_string());

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::Reparse(new)
    );
}

#[test]
fn identical_configuration_plans_none() {
    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &CsvConfiguration::default()),
        FormatChange::None
    );
}

#[test]
fn encoding_change_plans_reparse() {
    let new = changed(|configuration| configuration.encoding = Encoding::Windows1252);

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::Reparse(new)
    );
}

#[test]
fn escape_change_plans_reparse() {
    let new = changed(|configuration| configuration.escape_character = "\\".to_string());

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::Reparse(new)
    );
}

#[test]
fn parsing_change_wins_over_header_change() {
    let new = changed(|configuration| {
        configuration.column_separator = "\t".to_string();
        configuration.first_row_as_header = true;
    });

    assert_eq!(
        plan_format_change(&CsvConfiguration::default(), &new),
        FormatChange::Reparse(new)
    );
}

//! Row ordering for CSV tables.
//!
//! Cells are classified as numbers, text or empty. Numbers come first, text
//! follows using the locale's ICU collation with numeric ordering, and empty
//! cells always come last. Ties keep the original row order in both
//! directions.

use std::cmp::Ordering;

use icu_collator::options::{CollatorOptions, Strength};
use icu_collator::preferences::{CollationCaseFirst, CollationNumericOrdering};
use icu_collator::{Collator, CollatorBorrowed, CollatorPreferences};
use icu_locale_core::Locale;

use crate::{CsvTable, is_numeric};

/// Direction used by [`row_order_sorted_by`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Smallest key first.
    Ascending,
    /// Largest key first; empty keys still come last.
    Descending,
}

/// A cell's position in the sort order.
#[derive(Debug)]
pub(crate) enum CellKey {
    /// A numeric cell, parsed with the document decimal mark.
    Number(f64),
    /// A non-empty cell with its surrounding whitespace trimmed.
    Text(String),
    /// A blank cell, compared equal to every other blank cell.
    Empty,
}

/// Classifies a cell for sorting and filtering.
pub(crate) fn cell_key(cell: &str, decimal_mark: &str) -> CellKey {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return CellKey::Empty;
    }
    if is_numeric(trimmed, decimal_mark)
        && let Some(value) = parse_number(trimmed, decimal_mark)
    {
        return CellKey::Number(value);
    }
    CellKey::Text(trimmed.to_owned())
}

/// Parses an already trimmed numeric cell, replacing the document mark.
fn parse_number(trimmed: &str, decimal_mark: &str) -> Option<f64> {
    if decimal_mark.is_empty() {
        return trimmed.parse().ok();
    }
    trimmed.replace(decimal_mark, ".").parse().ok()
}

/// Builds the locale collator used for text, or `None` when its data is
/// unavailable.
///
/// The settings mirror Foundation's `localizedStandardCompare`: numeric
/// ordering, lowercase (and small kana) first, quaternary kana distinctions
/// and canonical-equivalence ties broken by the original code units.
fn make_collator(locale: &Locale) -> Option<CollatorBorrowed<'static>> {
    let mut preferences = CollatorPreferences::from(locale);
    preferences.numeric_ordering = Some(CollationNumericOrdering::True);
    preferences.case_first = Some(CollationCaseFirst::Lower);
    let mut options = CollatorOptions::default();
    options.strength = Some(Strength::Quaternary);
    Collator::try_new(preferences, options).ok()
}

/// Returns the row indexes ordered by `column`.
///
/// An invalid column yields the identity order. Empty cells sort last in both
/// directions and compare equal to each other. Text comparison uses `locale`
/// and falls back to ordinal comparison when the collator cannot be built.
pub fn row_order_sorted_by(
    table: &CsvTable,
    column: usize,
    direction: SortDirection,
    decimal_mark: &str,
    locale: &Locale,
) -> Vec<usize> {
    let row_count = table.row_count();
    let Some(column) = table.columns.get(column) else {
        return (0..row_count).collect();
    };

    let keys: Vec<CellKey> = column
        .cells
        .iter()
        .map(|cell| cell_key(cell, decimal_mark))
        .collect();
    let collator = make_collator(locale);
    let mut order: Vec<usize> = (0..row_count).collect();
    order.sort_by(|&lhs, &rhs| {
        compare(&keys[lhs], &keys[rhs], direction, collator.as_ref()).then_with(|| lhs.cmp(&rhs))
    });
    order
}

/// Three-way comparison, with empty keys last regardless of direction.
fn compare(
    lhs: &CellKey,
    rhs: &CellKey,
    direction: SortDirection,
    collator: Option<&CollatorBorrowed<'_>>,
) -> Ordering {
    match (lhs, rhs) {
        (CellKey::Empty, CellKey::Empty) => Ordering::Equal,
        (CellKey::Empty, _) => Ordering::Greater,
        (_, CellKey::Empty) => Ordering::Less,
        _ => {
            let order = ascending(lhs, rhs, collator);
            match direction {
                SortDirection::Ascending => order,
                SortDirection::Descending => order.reverse(),
            }
        }
    }
}

/// Comparison of two non-empty keys before the direction is applied.
fn ascending(lhs: &CellKey, rhs: &CellKey, collator: Option<&CollatorBorrowed<'_>>) -> Ordering {
    match (lhs, rhs) {
        (CellKey::Number(lhs), CellKey::Number(rhs)) => {
            lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal)
        }
        (CellKey::Text(lhs), CellKey::Text(rhs)) => collator.map_or_else(
            || lhs.cmp(rhs),
            |collator| match collator.compare(lhs.as_str(), rhs.as_str()) {
                Ordering::Equal => forced_order(lhs, rhs),
                order => order,
            },
        ),
        (CellKey::Number(_), _) => Ordering::Less,
        (_, CellKey::Number(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// Foundation breaks canonical-equivalence ties with the original code units
/// (`kCFCompareForcedOrdering`), which puts a decomposed sequence before its
/// composed form.
fn forced_order(lhs: &str, rhs: &str) -> Ordering {
    lhs.encode_utf16().cmp(rhs.encode_utf16())
}

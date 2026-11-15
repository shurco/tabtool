//! Row filters for CSV tables.
//!
//! Every active criterion must match a row (AND). A criterion is skipped when
//! it is disabled, has no known column, has an empty value while its operator
//! needs one, or carries an invalid regular expression.

use std::cmp::Ordering;

use regex::{Regex, RegexBuilder};
use unicode_normalization::UnicodeNormalization;

use crate::sort::{CellKey, cell_key};
use crate::{ColumnId, CriterionId, RowSource};

/// How a [`FilterCriterion`] compares a cell with its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterOperator {
    /// Numeric when both sides are numbers, otherwise case-insensitive text.
    Equals,
    /// The negation of [`FilterOperator::Equals`].
    NotEquals,
    /// Case-insensitive substring anywhere in the cell.
    Contains,
    /// The negation of [`FilterOperator::Contains`].
    NotContains,
    /// Case-insensitive prefix.
    BeginsWith,
    /// Case-insensitive suffix.
    EndsWith,
    /// Numeric or case-insensitive text ordering.
    GreaterThan,
    /// Numeric or case-insensitive text ordering.
    LessThan,
    /// Numeric or case-insensitive text ordering.
    GreaterOrEqual,
    /// Numeric or case-insensitive text ordering.
    LessOrEqual,
    /// Blank after trimming; ignores the criterion value.
    IsEmpty,
    /// Not blank after trimming; ignores the criterion value.
    IsNotEmpty,
    /// Case-insensitive regular expression, matching anywhere in the cell.
    MatchesRegex,
}

impl FilterOperator {
    /// Every operator, in the order the filter bar lists them.
    pub const ALL: [Self; 13] = [
        Self::Equals,
        Self::NotEquals,
        Self::Contains,
        Self::NotContains,
        Self::BeginsWith,
        Self::EndsWith,
        Self::GreaterThan,
        Self::LessThan,
        Self::GreaterOrEqual,
        Self::LessOrEqual,
        Self::IsEmpty,
        Self::IsNotEmpty,
        Self::MatchesRegex,
    ];

    /// Whether the operator compares against the criterion value.
    ///
    /// Only [`FilterOperator::IsEmpty`] and [`FilterOperator::IsNotEmpty`] do
    /// not need a value.
    pub fn needs_value(self) -> bool {
        !matches!(self, Self::IsEmpty | Self::IsNotEmpty)
    }
}

/// One row of the filter bar: a column, an operator and a value to compare.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterCriterion {
    /// Identity of the criterion in the document layer.
    pub id: CriterionId,
    /// Disabled criteria never narrow the rows.
    pub is_enabled: bool,
    /// Target column, or `None` when no column is chosen yet.
    pub column_id: Option<ColumnId>,
    /// Comparison operator.
    pub op: FilterOperator,
    /// Value to compare against; ignored by the emptiness operators.
    pub value: String,
}

impl FilterCriterion {
    /// Whether the criterion can be used.
    ///
    /// A regex with an empty or compilable pattern is valid; other operators
    /// accept any value, including an empty one.
    pub fn is_valid(&self) -> bool {
        self.op != FilterOperator::MatchesRegex
            || self.value.is_empty()
            || compile_regex(&self.value).is_some()
    }
}

/// A set of criteria that a row must all satisfy (AND).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowFilter {
    /// Criteria, in display order.
    pub criteria: Vec<FilterCriterion>,
}

impl RowFilter {
    /// Ascending indexes of the rows of `source` that satisfy every active
    /// criterion; all rows when none is active.
    ///
    /// `decimal_mark` controls numeric parsing, like sorting does. Foundation's
    /// case-insensitive text matching is locale-independent, so no locale is
    /// needed.
    pub fn matching_rows(&self, source: &dyn RowSource, decimal_mark: &str) -> Vec<usize> {
        let matchers = self.active_matchers(source, decimal_mark);
        (0..source.row_count())
            .filter(|&row| {
                matchers
                    .iter()
                    .all(|matcher| matcher.matches(&source.cell(row, matcher.column)))
            })
            .collect()
    }

    /// Whether at least one criterion would narrow the rows of `source`.
    pub fn is_active(&self, source: &dyn RowSource, decimal_mark: &str) -> bool {
        !self.active_matchers(source, decimal_mark).is_empty()
    }

    /// Builds one matcher per usable criterion, resolving column ids once.
    fn active_matchers(&self, source: &dyn RowSource, decimal_mark: &str) -> Vec<ColumnMatcher> {
        let column_ids = source.column_ids();
        self.criteria
            .iter()
            .filter_map(|criterion| {
                if !criterion.is_enabled {
                    return None;
                }
                if criterion.op.needs_value() && criterion.value.is_empty() {
                    return None;
                }
                let column = column_ids
                    .iter()
                    .position(|&id| Some(id) == criterion.column_id)?;
                let matcher = CellMatcher::new(criterion, decimal_mark)?;
                Some(ColumnMatcher { column, matcher })
            })
            .collect()
    }
}

/// A matcher bound to the column it reads.
struct ColumnMatcher {
    column: usize,
    matcher: CellMatcher,
}

impl ColumnMatcher {
    fn matches(&self, cell: &str) -> bool {
        self.matcher.matches(cell)
    }
}

/// The per-cell predicate of one criterion.
///
/// The value, the number and the regular expression are prepared once, not
/// per cell. [`CellMatcher::new`] returns `None` for an invalid regex.
enum CellMatcher {
    IsEmpty,
    IsNotEmpty,
    Contains(String),
    NotContains(String),
    BeginsWith(String),
    EndsWith(String),
    Regex(Regex),
    Comparison {
        op: FilterOperator,
        comparer: Comparer,
    },
}

impl CellMatcher {
    fn new(criterion: &FilterCriterion, decimal_mark: &str) -> Option<Self> {
        let value = criterion.value.as_str();
        Some(match criterion.op {
            FilterOperator::IsEmpty => Self::IsEmpty,
            FilterOperator::IsNotEmpty => Self::IsNotEmpty,
            FilterOperator::Contains => Self::Contains(folded(value)),
            FilterOperator::NotContains => Self::NotContains(folded(value)),
            FilterOperator::BeginsWith => Self::BeginsWith(folded(value)),
            FilterOperator::EndsWith => Self::EndsWith(folded(value)),
            FilterOperator::MatchesRegex => Self::Regex(compile_regex(value)?),
            FilterOperator::Equals
            | FilterOperator::NotEquals
            | FilterOperator::GreaterThan
            | FilterOperator::LessThan
            | FilterOperator::GreaterOrEqual
            | FilterOperator::LessOrEqual => Self::Comparison {
                op: criterion.op,
                comparer: Comparer::new(value, decimal_mark),
            },
        })
    }

    fn matches(&self, cell: &str) -> bool {
        match self {
            Self::IsEmpty => is_blank(cell),
            Self::IsNotEmpty => !is_blank(cell),
            Self::Contains(needle) => folded(cell).contains(needle),
            Self::NotContains(needle) => !folded(cell).contains(needle),
            Self::BeginsWith(needle) => folded(cell).starts_with(needle),
            Self::EndsWith(needle) => folded(cell).ends_with(needle),
            Self::Regex(regex) => regex.is_match(cell),
            Self::Comparison { op, comparer } => match comparer.order(cell) {
                Some(order) => compare(order, *op),
                // A blank cell satisfies no comparison, but it is not equal.
                None => *op == FilterOperator::NotEquals,
            },
        }
    }
}

/// Orders cells against one criterion value: numerically when both sides are
/// numbers, otherwise as case-insensitive text.
struct Comparer {
    number: Option<f64>,
    text: String,
    decimal_mark: String,
}

impl Comparer {
    fn new(value: &str, decimal_mark: &str) -> Self {
        let trimmed = value.trim();
        let number = match cell_key(trimmed, decimal_mark) {
            CellKey::Number(number) => Some(number),
            CellKey::Text(_) | CellKey::Empty => None,
        };
        Self {
            number,
            text: folded(trimmed),
            decimal_mark: decimal_mark.to_owned(),
        }
    }

    /// How `cell` orders relative to the value; `None` for a blank cell, which
    /// satisfies no comparison.
    fn order(&self, cell: &str) -> Option<Ordering> {
        match cell_key(cell, &self.decimal_mark) {
            CellKey::Empty => None,
            CellKey::Number(parsed) => match self.number {
                Some(number) => Some(parsed.partial_cmp(&number).unwrap_or(Ordering::Equal)),
                None => Some(folded(cell.trim()).cmp(&self.text)),
            },
            CellKey::Text(text) => Some(folded(&text).cmp(&self.text)),
        }
    }
}

/// Applies an operator to a three-way comparison.
fn compare(order: Ordering, op: FilterOperator) -> bool {
    match op {
        FilterOperator::Equals => order == Ordering::Equal,
        FilterOperator::NotEquals => order != Ordering::Equal,
        FilterOperator::GreaterThan => order == Ordering::Greater,
        FilterOperator::LessThan => order == Ordering::Less,
        FilterOperator::GreaterOrEqual => order != Ordering::Less,
        FilterOperator::LessOrEqual => order != Ordering::Greater,
        _ => false,
    }
}

/// Case-insensitive text folding with canonical equivalence.
///
/// Foundation compares text case-insensitively and treats composed and
/// decomposed forms as equal, so both the cell and the criterion value are
/// normalized to NFC before they are folded.
fn folded(text: &str) -> String {
    let normalized: String = text.nfc().collect();
    normalized.to_lowercase().nfc().collect()
}

/// Whether a cell is blank after trimming, like Foundation's whitespace sets.
fn is_blank(cell: &str) -> bool {
    cell.trim().is_empty()
}

/// Compiles a case-insensitive regex; invalid patterns yield `None`.
fn compile_regex(pattern: &str) -> Option<Regex> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .ok()
}

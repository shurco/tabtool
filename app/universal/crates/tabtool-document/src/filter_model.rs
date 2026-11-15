//! The row filter of one document: the criteria being edited, the filter that
//! is applied, and the rows it lets through.
//!
//! The draft `criteria` are edited freely and affect nothing until applied.
//! The applied filter is a snapshot: `visible_rows` is evaluated by an apply
//! or a refresh, never by a cell edit, so rows do not vanish while they are
//! typed into. The state is per document and is not undoable.
//!
//! The document side owns availability, evaluation against its table, the
//! row-structure lock that applies while a filter is active, the filtered
//! table used by export, and the selection trim after an apply.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use tabtool_core::{ColumnId, CriterionId, FilterCriterion, FilterOperator, RowFilter, RowSource};

mod document;

static NEXT_CRITERION_ID: AtomicU64 = AtomicU64::new(1);

/// One draft filter row with its own identity.
fn new_criterion() -> FilterCriterion {
    FilterCriterion {
        id: NEXT_CRITERION_ID.fetch_add(1, Ordering::Relaxed),
        is_enabled: true,
        column_id: None,
        op: FilterOperator::Contains,
        value: String::new(),
    }
}

/// The row filter of one document: the draft criteria, the applied filter and
/// the rows it lets through.
///
/// The state is view state, not document content: it has no undo entry and
/// does not bump the revision.
#[derive(Clone, Debug)]
pub struct RowFilterModel {
    /// The filter bar rows. Never empty.
    pub criteria: Vec<FilterCriterion>,
    /// The applied filter, or `None` when the whole table is visible.
    pub applied: Option<RowFilter>,
    /// Ascending model indexes of the rows that pass `applied`; `None` when no
    /// filter is applied.
    pub visible_rows: Option<Vec<usize>>,
    /// Whether the filter bar is shown; hiding it keeps an applied filter.
    pub is_bar_visible: bool,
    /// The columns of the source at the last apply or refresh.
    known_column_ids: Vec<ColumnId>,
}

impl Default for RowFilterModel {
    fn default() -> Self {
        Self {
            criteria: vec![new_criterion()],
            applied: None,
            visible_rows: None,
            is_bar_visible: false,
            known_column_ids: Vec::new(),
        }
    }
}

impl RowFilterModel {
    /// Whether an applied filter currently narrows the rows.
    pub fn is_filtering(&self) -> bool {
        self.applied.is_some()
    }

    /// Applies every enabled criterion of the draft (AND).
    pub fn apply_all(&mut self, source: &dyn RowSource, decimal_mark: &str) {
        let filter = RowFilter {
            criteria: self.criteria.clone(),
        };
        self.evaluate(filter, source, decimal_mark);
    }

    /// Applies one draft criterion alone, even when its checkbox is off.
    pub fn apply(&mut self, criterion_id: CriterionId, source: &dyn RowSource, decimal_mark: &str) {
        let Some(mut criterion) = self
            .criteria
            .iter()
            .find(|criterion| criterion.id == criterion_id)
            .cloned()
        else {
            return;
        };
        criterion.is_enabled = true;
        self.evaluate(
            RowFilter {
                criteria: vec![criterion],
            },
            source,
            decimal_mark,
        );
    }

    /// Removes the applied filter and resets the draft to one empty row.
    pub fn clear(&mut self) {
        self.applied = None;
        self.visible_rows = None;
        self.criteria = vec![new_criterion()];
    }

    /// Inserts an empty criterion after `after`, or at the end for an unknown
    /// id.
    pub fn add_criterion(&mut self, after: CriterionId) {
        let index = self
            .criteria
            .iter()
            .position(|criterion| criterion.id == after)
            .map_or(self.criteria.len(), |index| index + 1);
        self.criteria.insert(index, new_criterion());
    }

    /// Gives every draft criterion that has no column the column `id`, so a
    /// new row starts on a real column instead of matching nothing.
    pub fn fill_missing_columns(&mut self, id: ColumnId) {
        for criterion in &mut self.criteria {
            if criterion.column_id.is_none() {
                criterion.column_id = Some(id);
            }
        }
    }

    /// Removes a criterion; the last remaining row is reset instead, so the
    /// bar always has a row to edit.
    pub fn remove_criterion(&mut self, id: CriterionId) {
        self.criteria.retain(|criterion| criterion.id != id);
        if self.criteria.is_empty() {
            self.criteria = vec![new_criterion()];
        }
    }

    /// Re-evaluates the applied filter against the current rows and forgets
    /// columns that no longer exist: a draft criterion loses its column, an
    /// applied one is dropped, and the filter ends when none is active.
    ///
    /// A reload gives every column a new identity; that case keeps the
    /// criteria on the columns at the same positions.
    pub fn refresh(&mut self, source: &dyn RowSource, decimal_mark: &str) {
        let column_ids = source.column_ids();
        self.remap_replaced_columns(&column_ids);
        let remaining: HashSet<ColumnId> = column_ids.into_iter().collect();
        let is_missing = |criterion: &FilterCriterion| {
            criterion
                .column_id
                .is_some_and(|id| !remaining.contains(&id))
        };
        for criterion in &mut self.criteria {
            if is_missing(criterion) {
                criterion.column_id = None;
            }
        }
        let Some(mut filter) = self.applied.take() else {
            return;
        };
        filter.criteria.retain(|criterion| !is_missing(criterion));
        self.evaluate(filter, source, decimal_mark);
    }

    /// Moves the criteria to the new columns when none of the known columns
    /// survived and the column count did not change.
    fn remap_replaced_columns(&mut self, column_ids: &[ColumnId]) {
        let known = std::mem::replace(&mut self.known_column_ids, column_ids.to_vec());
        if known.is_empty() || known.len() != column_ids.len() {
            return;
        }
        let known_ids: HashSet<ColumnId> = known.iter().copied().collect();
        if column_ids.iter().any(|id| known_ids.contains(id)) {
            return;
        }
        let replacements: HashMap<ColumnId, ColumnId> =
            known.into_iter().zip(column_ids.iter().copied()).collect();
        for criterion in &mut self.criteria {
            remap_column(criterion, &replacements);
        }
        if let Some(filter) = &mut self.applied {
            for criterion in &mut filter.criteria {
                remap_column(criterion, &replacements);
            }
        }
    }

    /// Applies `filter`, or removes the applied filter when no criterion of it
    /// is active, so an empty bar never reports a filtered view.
    fn evaluate(&mut self, filter: RowFilter, source: &dyn RowSource, decimal_mark: &str) {
        self.known_column_ids = source.column_ids();
        if !filter.is_active(source, decimal_mark) {
            self.applied = None;
            self.visible_rows = None;
            return;
        }
        self.visible_rows = Some(filter.matching_rows(source, decimal_mark));
        self.applied = Some(filter);
    }
}

/// Moves one criterion to its replacement column, if any.
fn remap_column(criterion: &mut FilterCriterion, replacements: &HashMap<ColumnId, ColumnId>) {
    criterion.column_id = criterion
        .column_id
        .and_then(|id| replacements.get(&id).copied());
}

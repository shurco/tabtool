//! Row and column dragging.
//!
//! A column drag is keyed by [`ColumnId`], so a model reorder between the drag
//! start and its drop still moves the column the user grabbed. A row drag keeps
//! the display rows and builds its TSV payload on demand; drag-out of the window
//! is out of scope, but the payload is what an OS drag bridge would hand to an
//! external app. Only a drag held in [`GridState`] is accepted as a drop.

use std::collections::BTreeSet;

use egui::Stroke;
use tabtool_core::{ColumnId, RowSource, make_tsv};

use crate::theme::current;

use super::GridAction;
use super::geometry::{GridGeometry, HEADER_HEIGHT};
use super::paint::{Canvas, data_clip};
use super::state::GridState;

/// A column drag in flight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnDrag {
    /// The stable identity of the grabbed column.
    pub column: ColumnId,
    /// The insertion slot the pointer currently points at.
    pub target: usize,
}

impl ColumnDrag {
    /// Starts dragging the header at `column`.
    pub fn new(source: &dyn RowSource, column: usize) -> Option<Self> {
        let id = source.column_ids().get(column).copied()?;
        Some(Self {
            column: id,
            target: column,
        })
    }
}

/// A row drag in flight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowDrag {
    /// The display rows being dragged.
    pub rows: BTreeSet<usize>,
    /// The insertion row the pointer currently points at.
    pub target: Option<usize>,
}

impl RowDrag {
    /// Starts dragging `rows`, dropping rows the source no longer has.
    pub fn new(source: &dyn RowSource, rows: BTreeSet<usize>) -> Option<Self> {
        let row_count = source.row_count();
        let rows: BTreeSet<usize> = rows.into_iter().filter(|row| *row < row_count).collect();
        if rows.is_empty() {
            return None;
        }
        Some(Self { rows, target: None })
    }

    /// The rows as TSV, in visual column order.
    ///
    /// Numeric cells are localized from the document mark to the user's
    /// separator, exactly like a clipboard copy.
    pub fn payload(
        &self,
        source: &dyn RowSource,
        decimal_mark: &str,
        decimal_separator: &str,
    ) -> String {
        let row_count = source.row_count();
        let values: Vec<Vec<String>> = self
            .rows
            .iter()
            .filter(|row| **row < row_count)
            .map(|&row| {
                (0..source.column_count())
                    .map(|column| source.cell(row, column))
                    .collect()
            })
            .collect();
        make_tsv(&values, decimal_mark, decimal_separator)
    }
}

/// The insertion slot under `content_x`.
///
/// A pointer over the left half of a column inserts before it; the right half
/// inserts after it.
pub fn column_slot_at(geometry: &GridGeometry, content_x: f32) -> usize {
    if content_x <= geometry.gutter() {
        return 0;
    }
    let mut left = geometry.gutter();
    for (index, width) in geometry.widths.iter().copied().enumerate() {
        if content_x < left + width / 2.0 {
            return index;
        }
        left += width;
    }
    geometry.column_count()
}

/// The insertion row under `content_y`, clamped to the row count.
pub fn row_slot_at(geometry: &GridGeometry, content_y: f32) -> usize {
    if geometry.row_height <= 0.0 {
        return 0;
    }
    let delta = (content_y - HEADER_HEIGHT).max(0.0);
    let slot = (delta / geometry.row_height).round();
    slot.clamp(0.0, geometry.row_count as f32) as usize
}

/// The move a dropped column requests, if it changes the order.
pub fn column_drop_action(source: &dyn RowSource, drag: &ColumnDrag) -> Option<GridAction> {
    let column_count = source.column_count();
    let from = source
        .column_ids()
        .iter()
        .position(|id| *id == drag.column)?;
    let to = drag.target.min(column_count);
    (to != from && to != from + 1).then_some(GridAction::MoveColumn { from, to })
}

/// The move a dropped row drag requests, if it has a target.
pub fn row_drop_action(drag: &RowDrag, row_count: usize) -> Option<GridAction> {
    let target = drag.target?;
    if drag.rows.is_empty() {
        return None;
    }
    Some(GridAction::MoveRows {
        from: drag.rows.clone(),
        to: target.min(row_count),
    })
}

/// Paints the insertion indicator for the active drag.
pub(crate) fn paint_drop_indicator(canvas: &Canvas, geometry: &GridGeometry, state: &GridState) {
    let theme = current(canvas.painter.ctx());
    let color = theme.colors.accent;
    if let Some(drag) = &state.column_drag {
        let x = canvas.origin.x + geometry.column_x(drag.target.min(geometry.column_count()));
        if x >= canvas.clip.min.x && x <= canvas.clip.max.x {
            canvas
                .painter
                .vline(x, canvas.clip.y_range(), Stroke::new(2.0, color));
        }
    }
    if let Some(drag) = &state.row_drag
        && let Some(target) = drag.target
    {
        let data = data_clip(canvas.clip, geometry);
        let y = canvas.origin.y
            + HEADER_HEIGHT
            + target.min(geometry.row_count) as f32 * geometry.row_height;
        if data.width() > 0.0 && y >= data.min.y && y <= canvas.clip.max.y {
            canvas
                .painter
                .hline(data.x_range(), y, Stroke::new(2.0, color));
        }
    }
}

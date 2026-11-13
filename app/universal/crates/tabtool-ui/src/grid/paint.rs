//! Painting of the scrollable cell area and the read-once visible-cell buffer.

use core::ops::Range;

use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, pos2};
use tabtool_core::RowSource;

use crate::theme::current;

use super::geometry::{GridGeometry, HEADER_HEIGHT};
use super::options::GridOptions;
use super::state::{CellAlignment, GridState};

/// One cell read from the source for this frame.
#[derive(Clone, Debug)]
pub(crate) struct VisibleCell {
    /// Display row.
    pub row: usize,
    /// Visual column.
    pub column: usize,
    /// Frame in content coordinates.
    pub rect: Rect,
    /// Cell text.
    pub text: String,
    /// Whether the text is numeric and right-aligned.
    pub right_aligned: bool,
}

/// The cells whose frames touch the viewport plus overscan.
#[derive(Clone, Debug)]
pub(crate) struct VisibleCells {
    /// Rows that were read.
    pub rows: Range<usize>,
    /// Columns that were read.
    pub columns: Range<usize>,
    /// The read cells, row-major.
    pub cells: Vec<VisibleCell>,
}

impl VisibleCells {
    /// Reads every visible cell exactly once.
    pub fn read(
        source: &dyn RowSource,
        geometry: &GridGeometry,
        viewport: Rect,
        decimal_mark: &str,
    ) -> Self {
        let rows = geometry.visible_rows(viewport);
        let columns = geometry.visible_columns(viewport);
        let mut cells = Vec::with_capacity(rows.len().saturating_mul(columns.len()));
        for row in rows.clone() {
            for column in columns.clone() {
                let text = source.cell(row, column);
                cells.push(VisibleCell {
                    row,
                    column,
                    rect: geometry.cell_rect(row, column),
                    right_aligned: CellAlignment::of(&text, decimal_mark) == CellAlignment::Right,
                    text,
                });
            }
        }
        Self {
            rows,
            columns,
            cells,
        }
    }
}

/// The painter and layout of the current scroll viewport.
///
/// The painter is owned so the canvas can outlive the immutable borrow of the
/// UI that created it, while the render pass still creates click targets.
#[derive(Clone)]
pub(crate) struct Canvas {
    /// The painter clipped to the viewport.
    pub painter: Painter,
    /// The screen position of content coordinate zero.
    pub origin: Pos2,
    /// The screen rect available to the grid.
    pub clip: Rect,
    /// The visible rect in content coordinates.
    pub viewport: Rect,
}

impl Canvas {
    /// Captures the current viewport layout.
    pub fn new(painter: &Painter, origin: Pos2, clip: Rect, viewport: Rect) -> Self {
        Self {
            painter: painter.clone(),
            origin,
            clip,
            viewport,
        }
    }
}

/// The data area left after the pinned gutter and header.
pub(crate) fn data_clip(clip: Rect, geometry: &GridGeometry) -> Rect {
    let min = pos2(
        (clip.min.x + geometry.gutter()).min(clip.max.x),
        (clip.min.y + HEADER_HEIGHT).min(clip.max.y),
    );
    Rect::from_min_max(min, clip.max)
}

/// Paints the scrollable cell area.
pub(crate) fn paint_data(
    canvas: &Canvas,
    geometry: &GridGeometry,
    options: &GridOptions,
    state: &GridState,
    visible: &VisibleCells,
    grid_focused: bool,
) {
    let painter = &canvas.painter;
    let content_origin = canvas.origin;
    let theme = current(painter.ctx());
    let data = data_clip(canvas.clip, geometry);
    if data.width() <= 0.0 || data.height() <= 0.0 {
        return;
    }
    let offset = content_origin.to_vec2();

    for row in visible.rows.clone() {
        let top = content_origin.y + HEADER_HEIGHT + row as f32 * geometry.row_height;
        let row_rect = Rect::from_min_max(
            pos2(data.min.x, top),
            pos2(data.max.x, top + geometry.row_height),
        );
        if options.alternating_rows && row % 2 == 1 {
            painter.rect_filled(row_rect, 0, theme.colors.alternating_row_background);
        }
        if state.selection.rows.contains(&row) {
            painter.rect_filled(row_rect, 0, theme.colors.selection_background);
        }
    }

    for column in visible.columns.clone() {
        if state.selection.columns.contains(&column) {
            let left = content_origin.x + geometry.column_x(column);
            let rect = Rect::from_min_max(
                pos2(left, data.min.y),
                pos2(left + geometry.column_width(column), data.max.y),
            );
            painter.rect_filled(rect, 0, tint(theme.colors.accent));
        }
    }

    if let Some((row, column)) = state.hovered
        && visible.rows.contains(&row)
        && visible.columns.contains(&column)
        && !state.selection.rows.contains(&row)
        && !state.selection.columns.contains(&column)
    {
        let rect = geometry.cell_rect(row, column).translate(offset);
        painter.rect_filled(rect, 0, theme.colors.hover_background);
    }

    let text_font = FontId::proportional(theme.typography.body);
    for cell in &visible.cells {
        let rect = cell.rect.translate(offset);
        if !rect.intersects(data) || cell.text.is_empty() {
            continue;
        }
        let (position, align) = if cell.right_aligned {
            (
                pos2(rect.right() - 8.0, rect.center().y),
                Align2::RIGHT_CENTER,
            )
        } else {
            (
                pos2(rect.left() + 8.0, rect.center().y),
                Align2::LEFT_CENTER,
            )
        };
        painter.with_clip_rect(rect.intersect(data)).text(
            position,
            align,
            &cell.text,
            text_font.clone(),
            theme.colors.label,
        );
    }

    for column in visible.columns.clone() {
        let x = content_origin.x + geometry.column_x(column) + geometry.column_width(column) - 1.0;
        if x >= data.min.x && x <= data.max.x {
            painter.vline(x, data.y_range(), Stroke::new(1.0, theme.colors.separator));
        }
    }
    if options.shows_horizontal_lines() {
        for row in visible.rows.clone() {
            let y = content_origin.y + HEADER_HEIGHT + (row + 1) as f32 * geometry.row_height - 1.0;
            if y >= data.min.y && y <= data.max.y {
                painter.hline(data.x_range(), y, Stroke::new(1.0, theme.colors.separator));
            }
        }
    }

    if grid_focused
        && let Some(focus) = state.focus
        && visible.rows.contains(&focus.row)
        && visible.columns.contains(&focus.column)
    {
        let rect = geometry
            .cell_rect(focus.row, focus.column)
            .translate(offset)
            .intersect(data);
        painter.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(theme.metrics.focus_ring_width, theme.colors.focus_ring),
            StrokeKind::Inside,
        );
    }
}

/// A translucent accent fill for selected columns and headers.
pub(crate) fn tint(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40)
}

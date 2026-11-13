//! Painting of the pinned header row and row-number gutter.

use egui::{Align2, FontId, Painter, Rect, Shape, Stroke, pos2};
use tabtool_core::SortDirection;

use crate::theme::{Theme, current};

use super::geometry::{GridGeometry, HEADER_HEIGHT, SORT_ARROW_WIDTH};
use super::options::{GridOptions, SortSpec};
use super::paint::{Canvas, tint};
use super::state::{GridState, header_for};

/// Paints the pinned column headers.
pub(crate) fn paint_header(
    canvas: &Canvas,
    geometry: &GridGeometry,
    options: &GridOptions,
    state: &GridState,
    headers: Option<&[String]>,
) {
    let painter = &canvas.painter;
    let clip = canvas.clip;
    let viewport = canvas.viewport;
    let theme = current(painter.ctx());
    let left = (clip.min.x + geometry.gutter()).min(clip.max.x);
    let bottom = (clip.min.y + HEADER_HEIGHT).min(clip.max.y);
    let header = Rect::from_min_max(pos2(left, clip.min.y), pos2(clip.max.x, bottom));
    if header.width() <= 0.0 || header.height() <= 0.0 {
        return;
    }
    painter.rect_filled(header, 0, theme.colors.header_background);

    let font = FontId::proportional(theme.typography.caption);
    for column in geometry.visible_columns(viewport) {
        let rect = geometry
            .pinned_header_rect(column, viewport)
            .translate(clip.min.to_vec2());
        let visible = rect.intersect(header);
        if visible.width() <= 0.0 {
            continue;
        }
        if state.selection.columns.contains(&column) {
            painter.rect_filled(visible, 0, tint(theme.colors.accent));
            painter.rect_filled(
                Rect::from_min_max(pos2(visible.min.x, visible.max.y - 2.0), visible.max),
                0,
                theme.colors.accent,
            );
        }
        let text_clip = Rect::from_min_max(
            visible.min,
            pos2(
                (visible.max.x - SORT_ARROW_WIDTH).max(visible.min.x),
                visible.max.y,
            ),
        );
        painter.with_clip_rect(text_clip).text(
            pos2(visible.min.x + 8.0, visible.center().y),
            Align2::LEFT_CENTER,
            header_for(headers, column),
            font.clone(),
            theme.colors.label,
        );
        paint_sort_arrows(painter, rect, options.sort, column, &theme);
        painter.vline(
            rect.right() - 1.0,
            clip.min.y..=bottom,
            Stroke::new(1.0, theme.colors.separator),
        );
    }
    painter.hline(
        header.x_range(),
        bottom - 1.0,
        Stroke::new(1.0, theme.colors.separator),
    );
}

/// Paints the pinned row-number gutter and its corner.
pub(crate) fn paint_gutter(
    canvas: &Canvas,
    geometry: &GridGeometry,
    options: &GridOptions,
    state: &GridState,
) {
    let painter = &canvas.painter;
    let clip = canvas.clip;
    let viewport = canvas.viewport;
    let theme = current(painter.ctx());
    let right = (clip.min.x + geometry.gutter()).min(clip.max.x);
    if right <= clip.min.x {
        return;
    }
    let corner = geometry
        .pinned_corner_rect()
        .translate(clip.min.to_vec2())
        .intersect(clip);
    painter.rect_filled(corner, 0, theme.colors.header_background);
    let rows_clip = Rect::from_min_max(
        pos2(clip.min.x, (clip.min.y + HEADER_HEIGHT).min(clip.max.y)),
        pos2(right, clip.max.y),
    );
    let rows_painter = painter.with_clip_rect(rows_clip);

    for row in geometry.visible_rows(viewport) {
        let rect = geometry
            .pinned_gutter_rect(row, viewport)
            .translate(clip.min.to_vec2())
            .intersect(Rect::from_min_max(clip.min, pos2(right, clip.max.y)));
        rows_painter.rect_filled(rect, 0, theme.colors.content_background);
        if options.alternating_rows && row % 2 == 1 {
            rows_painter.rect_filled(rect, 0, theme.colors.alternating_row_background);
        }
        if state.selection.rows.contains(&row) {
            rows_painter.rect_filled(rect, 0, theme.colors.selection_background);
        }
        if options.shows_horizontal_lines() && rect.max.y <= clip.max.y {
            rows_painter.hline(
                rect.x_range(),
                rect.max.y - 1.0,
                Stroke::new(1.0, theme.colors.separator),
            );
        }
        rows_painter.with_clip_rect(rect).text(
            pos2(right - 6.0, rect.center().y),
            Align2::RIGHT_CENTER,
            geometry.row_number(row).to_string(),
            FontId::monospace(theme.typography.caption),
            theme.colors.secondary_label,
        );
    }

    painter.vline(
        right - 1.0,
        clip.min.y..=clip.max.y,
        Stroke::new(1.0, theme.colors.separator),
    );
    if corner.height() > 0.0 {
        painter.hline(
            corner.x_range(),
            corner.max.y - 1.0,
            Stroke::new(1.0, theme.colors.separator),
        );
    }
}

fn paint_sort_arrows(
    painter: &Painter,
    rect: Rect,
    sort: Option<SortSpec>,
    column: usize,
    theme: &Theme,
) {
    let center = pos2(rect.right() - SORT_ARROW_WIDTH / 2.0 - 2.0, rect.center().y);
    let gap = 1.5;
    let half = 4.0;
    for (direction, side) in [
        (SortDirection::Ascending, -1.0),
        (SortDirection::Descending, 1.0),
    ] {
        let base = center.y + side * gap;
        let color = match sort {
            Some(sort) if sort.column == column && sort.direction == direction => {
                theme.colors.accent
            }
            _ => theme.colors.tertiary_label,
        };
        painter.add(Shape::convex_polygon(
            vec![
                pos2(center.x - half, base),
                pos2(center.x + half, base),
                pos2(center.x, base + side * half),
            ],
            color,
            Stroke::NONE,
        ));
    }
}

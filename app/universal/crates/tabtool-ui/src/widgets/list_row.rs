//! A two-line selectable row, e.g. for the Documents list.

use egui::{Align2, Color32, CornerRadius, FontId, Response, Sense, Stroke, Ui, Vec2, WidgetInfo};

use super::focus_ring;
use crate::theme::current;

/// Show a two-line clickable row.
pub fn list_row(ui: &mut Ui, title: &str, subtitle: &str, selected: bool) -> Response {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let height = theme.metrics.row_height + 12.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());

    let background = if selected {
        colors.selection_background
    } else if response.hovered() {
        colors.hover_background
    } else {
        Color32::TRANSPARENT
    };
    let radius = CornerRadius::same(theme.metrics.small_corner_radius);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, background);
    painter.text(
        rect.left_center() + Vec2::new(10.0, -7.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(theme.typography.body),
        colors.label,
    );
    painter.text(
        rect.left_center() + Vec2::new(10.0, 7.0),
        Align2::LEFT_CENTER,
        subtitle,
        FontId::proportional(theme.typography.caption),
        colors.secondary_label,
    );

    response
        .widget_info(|| WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), selected, title));
    if !selected && response.hovered() {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, colors.separator),
            egui::StrokeKind::Inside,
        );
    }
    focus_ring(ui, &response);
    response
}

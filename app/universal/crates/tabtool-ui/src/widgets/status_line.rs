//! The bottom status line: counts and the format summary.

use egui::{Align, Frame, Layout, Margin, Response, RichText, Stroke, Ui};

use crate::theme::current;

/// Show the status line with left-aligned detail and a right-aligned count.
pub fn status_line(ui: &mut Ui, left: &str, right: &str) -> Response {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let frame = Frame::new()
        .fill(colors.header_background)
        .inner_margin(Margin::symmetric(
            theme.metrics.status_padding_x as i8,
            theme.metrics.status_padding_y as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(theme.metrics.status_line_height - 2.0 * theme.metrics.status_padding_y);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(left)
                        .size(theme.typography.caption)
                        .color(colors.secondary_label),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(right)
                            .size(theme.typography.caption)
                            .color(colors.secondary_label),
                    );
                });
            });
        });

    let rect = frame.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0, colors.separator),
    );
    frame.response
}

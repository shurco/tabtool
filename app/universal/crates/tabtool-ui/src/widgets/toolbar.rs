//! The horizontal toolbar strip above the grid.

use egui::{Frame, InnerResponse, Margin, Sense, Stroke, Ui};

use crate::theme::current;

/// Show a toolbar row with the standard insets and a bottom separator.
pub fn toolbar<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let frame = Frame::new()
        .fill(colors.window_background)
        .inner_margin(Margin::symmetric(
            theme.metrics.toolbar_padding_x as i8,
            theme.metrics.toolbar_padding_y as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.set_height(theme.metrics.toolbar_height - 2.0 * theme.metrics.toolbar_padding_y);
                add_contents(ui)
            })
        });

    let rect = frame.response.rect;
    ui.painter().hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0, colors.separator),
    );

    InnerResponse::new(frame.inner.inner, frame.response)
}

/// A vertical hairline separating toolbar groups.
pub fn toolbar_separator(ui: &mut Ui) {
    let theme = current(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(1.0, ui.spacing().interact_size.y),
        Sense::hover(),
    );
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0, theme.colors.separator),
    );
}

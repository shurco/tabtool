//! The sheet container used by settings, filters and export options.

use core::hash::Hash;

use egui::{Align, Color32, CornerRadius, Frame, Id, Layout, Margin, RichText, Stroke, Ui};

use super::PushButton;
use crate::theme::current;

/// The result of showing a sheet.
pub struct SheetResponse<R> {
    /// What the sheet content returned.
    pub inner: R,
    /// Whether the close button was pressed.
    pub close_requested: bool,
}

/// Show a modal sheet with a title row and a close button.
pub fn sheet<R>(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    title: &str,
    close_label: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> SheetResponse<R> {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let mut close_requested = false;

    let frame = Frame::new()
        .fill(colors.control_background)
        .stroke(Stroke::new(1.0, colors.separator))
        .corner_radius(CornerRadius::same(theme.metrics.large_corner_radius))
        .inner_margin(Margin::same(theme.metrics.sheet_padding as i8))
        .shadow(ui.style().visuals.popup_shadow);

    let response = egui::Modal::new(Id::unique(id_salt))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(60))
        .show(ui.ctx(), |ui| {
            ui.set_width(theme.metrics.sheet_width);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .font(crate::fonts::semibold_font(theme.typography.title))
                        .color(colors.label),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if PushButton::new(close_label, super::ButtonKind::Bordered)
                        .show(ui)
                        .clicked()
                    {
                        close_requested = true;
                    }
                });
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(10.0);
            add_contents(ui)
        });

    SheetResponse {
        inner: response.inner,
        close_requested,
    }
}

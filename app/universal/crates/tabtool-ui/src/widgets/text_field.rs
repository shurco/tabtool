//! Single-line text fields and the search variant.

use egui::{Margin, Response, RichText, TextEdit, Ui, Vec2};

use super::focus_ring;
use crate::theme::current;

/// A single-line text field that fills the available width.
///
/// `hint` is the placeholder and must already be localized by the caller.
pub fn text_field(ui: &mut Ui, name: &str, text: &mut String, hint: &str) -> Response {
    let theme = current(ui.ctx());
    let width = ui.available_width();
    field(ui, name, text, hint, width, theme.typography.body)
}

/// A search field: a magnifier plus a text field.
pub fn search_field(ui: &mut Ui, name: &str, text: &mut String) -> Response {
    let theme = current(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("\u{1F50D}")
                .size(theme.typography.body)
                .color(theme.colors.secondary_label),
        );
        let width = ui.available_width();
        field(ui, name, text, "", width, theme.typography.body)
    })
    .inner
}

fn field(
    ui: &mut Ui,
    name: &str,
    text: &mut String,
    hint: &str,
    width: f32,
    font_size: f32,
) -> Response {
    let theme = current(ui.ctx());
    let response = ui
        .add(
            TextEdit::singleline(text)
                .hint_text(RichText::new(hint).color(theme.colors.tertiary_label))
                .font(egui::FontId::proportional(font_size))
                .margin(Margin::symmetric(6, 3))
                .desired_width(width)
                .min_size(Vec2::new(width, theme.metrics.control_height)),
        )
        .accessible_name(name);
    focus_ring(ui, &response);
    response
}

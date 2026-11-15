//! A segmented control, e.g. for the row-height choice.

use egui::{Button, Color32, CornerRadius, Frame, Margin, Response, RichText, Stroke, Ui, Vec2};

use super::focus_ring;
use crate::theme::current;

/// A horizontal group of mutually exclusive segments.
///
/// The caller owns the selection; a click updates `selected` and the returned
/// response reports it as changed.
pub fn segmented(ui: &mut Ui, options: &[&str], selected: &mut usize) -> Response {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let radius = CornerRadius::same(theme.metrics.corner_radius);
    let mut changed = false;

    let frame = Frame::new()
        .fill(colors.header_background)
        .stroke(Stroke::new(1.0, colors.control_border))
        .corner_radius(radius)
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (index, label) in options.iter().enumerate() {
                    let is_selected = index == *selected;
                    let mut style = ui.style().as_ref().clone();
                    style.visuals.widgets.inactive.weak_bg_fill = if is_selected {
                        colors.raised_background
                    } else {
                        Color32::TRANSPARENT
                    };
                    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                    style.visuals.widgets.hovered.weak_bg_fill = if is_selected {
                        colors.raised_background
                    } else {
                        colors.hover_background
                    };
                    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                    style.visuals.widgets.active.weak_bg_fill = colors.selection_background;
                    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
                    style.visuals.widgets.active.fg_stroke.color = colors.label;
                    style.visuals.widgets.inactive.corner_radius =
                        CornerRadius::same(theme.metrics.small_corner_radius);

                    let response = ui
                        .scope(|ui| {
                            ui.set_style(style);
                            ui.add(
                                Button::new(
                                    RichText::new(*label).size(theme.typography.body).color(
                                        if is_selected {
                                            colors.label
                                        } else {
                                            colors.secondary_label
                                        },
                                    ),
                                )
                                .frame(true)
                                .min_size(Vec2::new(0.0, theme.metrics.control_height - 4.0)),
                            )
                        })
                        .inner
                        .accessible_name(*label);

                    if response.clicked() {
                        *selected = index;
                        changed = true;
                    }
                    focus_ring(ui, &response);
                }
            });
        });

    let mut response = frame.response;
    if changed {
        response.mark_changed();
    }
    response
}

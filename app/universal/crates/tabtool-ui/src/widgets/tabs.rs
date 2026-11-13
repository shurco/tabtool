//! The tab bar used by the Settings window.

use egui::{Button, Color32, CornerRadius, Frame, Margin, Response, RichText, Stroke, Ui, Vec2};

use super::focus_ring;
use crate::theme::current;

/// Show a horizontal tab bar; a click updates `selected`.
pub fn tab_bar(ui: &mut Ui, tabs: &[&str], selected: &mut usize) -> Response {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let radius = CornerRadius::same(theme.metrics.small_corner_radius);
    let mut changed = false;

    let frame = Frame::new()
        .fill(colors.header_background)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (index, label) in tabs.iter().enumerate() {
                    let is_selected = index == *selected;
                    let mut style = ui.style().as_ref().clone();
                    style.visuals.widgets.inactive.weak_bg_fill = if is_selected {
                        colors.selection_background
                    } else {
                        Color32::TRANSPARENT
                    };
                    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                    style.visuals.widgets.inactive.fg_stroke.color = if is_selected {
                        colors.label
                    } else {
                        colors.secondary_label
                    };
                    style.visuals.widgets.hovered.weak_bg_fill = colors.hover_background;
                    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                    style.visuals.widgets.active.weak_bg_fill = colors.selection_background;
                    style.visuals.widgets.active.fg_stroke.color = colors.label;
                    style.visuals.widgets.inactive.corner_radius = radius;

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

                    if is_selected {
                        let underline = response.rect.x_range().shrink(4.0);
                        ui.painter().hline(
                            underline,
                            response.rect.bottom() - 1.0,
                            Stroke::new(2.0, colors.accent),
                        );
                    }
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

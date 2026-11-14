//! Modal alerts in the style of a macOS alert.

use core::hash::Hash;

use egui::{Align, Color32, CornerRadius, Frame, Id, Layout, Margin, RichText, Stroke, Ui};

use super::{ButtonKind, PushButton};
use crate::theme::current;

/// One button in an alert.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlertAction<'a> {
    /// The visible, localized label.
    pub label: &'a str,
    /// The visual role of the button.
    pub kind: ButtonKind,
    /// Whether the button can be pressed.
    pub enabled: bool,
}

impl<'a> AlertAction<'a> {
    /// A pressable action button.
    pub fn new(label: &'a str, kind: ButtonKind) -> Self {
        Self {
            label,
            kind,
            enabled: true,
        }
    }

    /// Enable or disable the action.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Show a modal alert; returns the index of the pressed action.
///
/// The first action is the default: it sits at the trailing edge and Return
/// activates it. The last action is the cancel action: Escape, a backdrop
/// click and the window close button all report it.
///
/// The alert stays visible until the caller hides it after receiving an index.
pub fn modal(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug,
    title: &str,
    message: &str,
    actions: &[AlertAction<'_>],
) -> Option<usize> {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let mut pressed = None;

    let frame = Frame::new()
        .fill(colors.control_background)
        .stroke(Stroke::new(1.0, colors.separator))
        .corner_radius(CornerRadius::same(theme.metrics.large_corner_radius))
        .inner_margin(Margin::same(theme.metrics.alert_padding as i8))
        .shadow(ui.style().visuals.popup_shadow);

    let response = egui::Modal::new(Id::unique(id_salt))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(60))
        .show(ui.ctx(), |ui| {
            ui.set_width(theme.metrics.alert_width);
            ui.label(
                RichText::new(title)
                    .font(crate::fonts::semibold_font(theme.typography.title))
                    .color(colors.label),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(message)
                    .size(theme.typography.body)
                    .color(colors.secondary_label),
            );
            ui.add_space(14.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // macOS puts the default action at the trailing edge and
                // Cancel at the leading edge.
                for (index, action) in actions.iter().enumerate() {
                    let response = PushButton::new(action.label, action.kind)
                        .enabled(action.enabled)
                        .show(ui);
                    if response.clicked() {
                        pressed = Some(index);
                    }
                }
            });
        });

    if pressed.is_none() {
        if response.should_close() {
            // Escape and a backdrop click cancel through the last action,
            // which every alert uses for Cancel or its single OK button.
            pressed = actions.len().checked_sub(1);
        } else if response.is_top_modal
            && ui
                .ctx()
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        {
            // Return activates the default (first) action.
            pressed = (!actions.is_empty()).then_some(0);
        }
    }

    pressed
}

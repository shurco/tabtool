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
/// The alert stays visible until the caller hides it after receiving an index.
pub fn alert(
    ui: &mut Ui,
    id_salt: impl Hash + std::fmt::Debug + std::fmt::Debug,
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
        .inner_margin(Margin::same(16))
        .shadow(ui.style().visuals.popup_shadow);

    egui::Modal::new(Id::unique(id_salt))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(60))
        .show(ui.ctx(), |ui| {
            ui.set_width(340.0);
            ui.label(
                RichText::new(title)
                    .size(theme.typography.title)
                    .strong()
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
                for (index, action) in actions.iter().enumerate().rev() {
                    let response = PushButton::new(action.label, action.kind)
                        .enabled(action.enabled)
                        .show(ui);
                    if response.clicked() {
                        pressed = Some(index);
                    }
                }
            });
        });

    pressed
}

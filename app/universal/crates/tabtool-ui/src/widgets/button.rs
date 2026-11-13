//! Push buttons in the four macOS variants.

use egui::{Button, Color32, Response, RichText, Stroke, Ui, Vec2};

use super::focus_ring;
use crate::theme::current;

/// The visual role of a push button.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonKind {
    /// No border or fill; used for toolbar and disclosure buttons.
    Plain,
    /// The standard bordered button.
    #[default]
    Bordered,
    /// The default action: an accent-filled button.
    Prominent,
    /// A destructive action: bordered with red text.
    Destructive,
}

/// A push button with an accessible name and a focus ring.
pub struct PushButton<'a> {
    label: &'a str,
    kind: ButtonKind,
    enabled: bool,
    selected: bool,
    accessible_name: Option<&'a str>,
    min_width: Option<f32>,
}

impl<'a> PushButton<'a> {
    /// A button with `label` in the given role.
    pub fn new(label: &'a str, kind: ButtonKind) -> Self {
        Self {
            label,
            kind,
            enabled: true,
            selected: false,
            accessible_name: None,
            min_width: None,
        }
    }

    /// Enable or disable the button.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Mark the button as toggled on.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Override the accessible name; defaults to the visible label.
    pub fn accessible_name(mut self, name: &'a str) -> Self {
        self.accessible_name = Some(name);
        self
    }

    /// Force a minimum width.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Show the button.
    pub fn show(self, ui: &mut Ui) -> Response {
        let theme = current(ui.ctx());
        let colors = theme.colors;
        let metrics = theme.metrics;
        let name = self.accessible_name.unwrap_or(self.label);

        let text_color = if self.enabled {
            match self.kind {
                ButtonKind::Prominent => colors.control_background,
                ButtonKind::Destructive => colors.error,
                ButtonKind::Plain | ButtonKind::Bordered => colors.label,
            }
        } else {
            colors.tertiary_label
        };

        let mut style = ui.style().as_ref().clone();
        match self.kind {
            ButtonKind::Plain => {
                style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                style.visuals.widgets.hovered.weak_bg_fill = colors.hover_background;
                style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
            }
            ButtonKind::Bordered => {}
            ButtonKind::Prominent => {
                style.visuals.widgets.inactive.weak_bg_fill = colors.accent;
                style.visuals.widgets.inactive.bg_fill = colors.accent;
                style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
                style.visuals.widgets.hovered.weak_bg_fill = colors.accent_pressed;
                style.visuals.widgets.hovered.bg_fill = colors.accent_pressed;
                style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
                style.visuals.widgets.hovered.fg_stroke.color = colors.control_background;
                style.visuals.widgets.active.weak_bg_fill = colors.accent_pressed;
                style.visuals.widgets.active.bg_fill = colors.accent_pressed;
                style.visuals.widgets.active.fg_stroke.color = colors.control_background;
            }
            ButtonKind::Destructive => {
                style.visuals.widgets.inactive.fg_stroke.color = colors.error;
                style.visuals.widgets.hovered.fg_stroke.color = colors.error;
                style.visuals.widgets.active.bg_fill = colors.error;
                style.visuals.widgets.active.weak_bg_fill = colors.error;
                style.visuals.widgets.active.fg_stroke.color = colors.control_background;
            }
        }

        let min_width = self.min_width.unwrap_or(match self.kind {
            ButtonKind::Plain => 0.0,
            _ => 50.0,
        });
        let height = if self.kind == ButtonKind::Plain {
            metrics.control_height
        } else {
            metrics.control_height_large
        };

        let response = ui
            .scope(|ui| {
                ui.set_style(style);
                let text = RichText::new(self.label)
                    .size(theme.typography.body)
                    .color(text_color);
                let mut button = Button::new(text)
                    .selected(self.selected)
                    .min_size(Vec2::new(min_width, height));
                if self.kind == ButtonKind::Plain {
                    button = button.frame(false);
                }
                ui.add_enabled(self.enabled, button)
            })
            .inner
            .accessible_name(name);

        focus_ring(ui, &response);
        response
    }
}

/// A bordered push button.
pub fn push_button(ui: &mut Ui, label: &str, kind: ButtonKind) -> Response {
    PushButton::new(label, kind).show(ui)
}

/// The default action button.
pub fn primary_button(ui: &mut Ui, label: &str) -> Response {
    PushButton::new(label, ButtonKind::Prominent).show(ui)
}

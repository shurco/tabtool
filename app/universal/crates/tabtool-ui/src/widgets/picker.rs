//! A popup picker in the style of a macOS pop-up button.

use core::hash::Hash;

use egui::{Align, Button, ComboBox, Id, Layout, RichText, Ui, Vec2};

use super::focus_ring;
use crate::theme::current;

/// A pop-up button with a visible selected value and a popup list.
pub struct Picker<'a> {
    id: Id,
    name: &'a str,
    selected: &'a str,
    options: &'a [&'a str],
    width: Option<f32>,
    height: Option<f32>,
    text_size: Option<f32>,
    enabled: bool,
}

impl<'a> Picker<'a> {
    /// A picker identified by `id_salt`.
    pub fn new(
        id_salt: impl Hash + std::fmt::Debug,
        name: &'a str,
        selected: &'a str,
        options: &'a [&'a str],
    ) -> Self {
        Self {
            id: Id::unique(id_salt),
            name,
            selected,
            options,
            width: None,
            height: None,
            text_size: None,
            enabled: true,
        }
    }

    /// Set the width of the closed button.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the height of the closed button.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the text size of the closed button.
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);
        self
    }

    /// Enable or disable the picker.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Show the picker; returns the chosen option index.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let theme = current(ui.ctx());
        let width = self.width.unwrap_or(ui.spacing().combo_width);
        let height = self.height.unwrap_or(theme.metrics.control_height_large);
        let text_size = self.text_size.unwrap_or(theme.typography.body);
        let mut chosen = None;

        let response = ui
            .add_enabled_ui(self.enabled, |ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(width, height),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_width(width);
                        let combo = ComboBox::from_id_salt(self.id)
                            .selected_text(RichText::new(self.selected).size(text_size))
                            .width(width)
                            .truncate();
                        combo
                            .show_ui(ui, |ui| {
                                ui.set_min_width(width);
                                for (index, option) in self.options.iter().enumerate() {
                                    let selected = self.selected == *option;
                                    let response = if self.height.is_some() {
                                        ui.add_sized(
                                            [ui.available_width(), height],
                                            Button::new(RichText::new(*option).size(text_size))
                                                .selected(selected),
                                        )
                                    } else {
                                        ui.selectable_label(selected, *option)
                                    };
                                    if response.clicked() {
                                        chosen = Some(index);
                                        ui.close();
                                    }
                                }
                            })
                            .response
                    },
                )
                .inner
            })
            .inner
            .accessible_name(self.name);

        focus_ring(ui, &response);
        chosen
    }
}

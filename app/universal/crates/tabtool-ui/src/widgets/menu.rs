//! In-window menu bar and menu items with shortcut hints.

use egui::{Button, MenuBar, Ui};

/// One menu item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuItem<'a> {
    /// Stable id returned when the item is chosen.
    pub id: &'a str,
    /// The visible, localized label.
    pub label: &'a str,
    /// The shortcut hint, e.g. `⌘S`.
    pub shortcut: Option<&'a str>,
    /// Whether the item can be chosen.
    pub enabled: bool,
    /// Whether the item is currently on.
    pub selected: bool,
}

impl<'a> MenuItem<'a> {
    /// An enabled, unselected item.
    pub fn new(id: &'a str, label: &'a str) -> Self {
        Self {
            id,
            label,
            shortcut: None,
            enabled: true,
            selected: false,
        }
    }

    /// Show a shortcut hint aligned to the right of the item.
    pub fn shortcut(mut self, shortcut: &'a str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Enable or disable the item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Mark the item as on.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// One top-level menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Menu<'a> {
    /// The visible, localized label.
    pub label: &'a str,
    /// The items in display order.
    pub items: &'a [MenuItem<'a>],
}

impl<'a> Menu<'a> {
    /// A menu with the given items.
    pub fn new(label: &'a str, items: &'a [MenuItem<'a>]) -> Self {
        Self { label, items }
    }
}

/// Show the in-window menu bar; returns the id of the chosen item.
pub fn menu_bar<'a>(ui: &mut Ui, menus: &'a [Menu<'a>]) -> Option<&'a str> {
    let mut chosen = None;
    MenuBar::new().ui(ui, |ui| {
        for menu in menus {
            let menu_button = ui.menu_button(menu.label, |ui| {
                for item in menu.items {
                    let mut button = Button::new(item.label)
                        .selected(item.selected)
                        .min_size(egui::Vec2::new(140.0, 0.0));
                    if let Some(shortcut) = item.shortcut {
                        button = button.shortcut_text(shortcut);
                    }
                    let response = ui
                        .add_enabled(item.enabled, button)
                        .accessible_name(item.label);
                    super::focus_ring(ui, &response);
                    if response.clicked() {
                        chosen = Some(item.id);
                        ui.close();
                    }
                }
            });
            super::focus_ring(ui, &menu_button.response);
        }
    });
    chosen
}

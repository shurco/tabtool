//! The editor accessory bar above the keyboard: Previous, Next, Done.

use egui::{Rect, Ui, Vec2, pos2};
use tabtool_i18n::{Language, tr};

use super::{MobileMetrics, TouchCell, bar_fill, hairline};

/// The action the accessory bar asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessoryAction {
    /// Commit the cell and move to the previous cell.
    Previous,
    /// Commit the cell and move to the next cell.
    Next,
    /// Commit the cell and close the editor.
    Done,
}

/// Draw the keyboard accessory bar; returns the pressed action.
///
/// `moves` draws Previous Cell and Next Cell. A column rename has no cells to
/// step through, so the caller hides them and keeps Done.
pub fn editor_accessory(
    ui: &mut Ui,
    metrics: MobileMetrics,
    language: Language,
    moves: bool,
) -> Option<AccessoryAction> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, metrics.accessory_height),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0.0, bar_fill(ui));
    hairline(ui, rect, rect.top());

    let inset = 4.0;
    let cell = metrics.control_height;
    let mut trailing = rect.right() - inset;
    let mut button = |ui: &Ui, glyph: &str, name: &str, salt: &str, action: AccessoryAction| {
        trailing -= cell;
        let cell_rect = Rect::from_min_size(
            pos2(trailing, rect.top()),
            Vec2::new(cell, metrics.accessory_height),
        );
        let response = TouchCell::new(glyph, name).show(ui, cell_rect, salt);
        response.clicked().then_some(action)
    };

    let mut chosen = None;
    if let Some(action) = button(
        ui,
        "\u{2713}",
        tr(language, "Done"),
        "editor-accessory-done",
        AccessoryAction::Done,
    ) {
        chosen = Some(action);
    }
    if moves {
        if let Some(action) = button(
            ui,
            "\u{203A}",
            tr(language, "Next Cell"),
            "editor-accessory-next",
            AccessoryAction::Next,
        ) {
            chosen = Some(action);
        }
        if let Some(action) = button(
            ui,
            "\u{2039}",
            tr(language, "Previous Cell"),
            "editor-accessory-previous",
            AccessoryAction::Previous,
        ) {
            chosen = Some(action);
        }
    }
    chosen
}

//! The reusable desktop widgets.

mod banner;
mod button;
mod list_row;
mod menu;
mod modal;
mod picker;
mod segmented;
mod sheet;
mod status_line;
mod tabs;
mod text_field;
mod toggle;
mod toolbar;

pub use banner::{BannerKind, BannerResponse, banner};
pub use button::{ButtonKind, PushButton, primary_button, push_button};
pub use list_row::list_row;
pub use menu::{Menu, MenuItem, menu_bar};
pub use modal::{AlertAction, modal};
pub use picker::Picker;
pub use segmented::segmented;
pub use sheet::{SheetResponse, sheet};
pub use status_line::status_line;
pub use tabs::tab_bar;
pub use text_field::{search_field, text_field};
pub use toggle::{toggle, toggle_enabled};
pub use toolbar::{toolbar, toolbar_separator};

use egui::{Response, Stroke, StrokeKind, Ui};

use crate::theme::current;

/// Draw the keyboard focus ring around `response`, if it has focus.
pub(crate) fn focus_ring(ui: &Ui, response: &Response) {
    if !response.has_focus() || !response.enabled() {
        return;
    }
    let theme = current(ui.ctx());
    let radius = egui::CornerRadius::same(theme.metrics.corner_radius.saturating_add(1));
    ui.painter().rect_stroke(
        response.rect.expand(1.0),
        radius,
        Stroke::new(theme.metrics.focus_ring_width, theme.colors.focus_ring),
        StrokeKind::Outside,
    );
}

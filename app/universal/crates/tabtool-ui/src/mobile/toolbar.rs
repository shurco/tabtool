//! The mobile navigation bar: back, history, title, filter, export, overflow.

use egui::{Button, Rect, RichText, Ui, Vec2, pos2};
use tabtool_i18n::{Language, tr};

use super::{MobileMetrics, TouchCell, bar_fill, hairline};
use crate::fonts::heading_font;
use crate::icons::Icon;
use crate::theme::current;

mod response;
mod title;

pub use response::{MobileToolbarResponse, OverflowItem};
use title::cached_elided_title;

/// The editor's navigation bar.
pub struct MobileToolbar<'a> {
    metrics: MobileMetrics,
    language: Language,
    title: &'a str,
    modified: bool,
    can_undo: bool,
    can_redo: bool,
    can_filter: bool,
    filter_count: usize,
    can_export: bool,
    can_share: bool,
    large: bool,
}

impl<'a> MobileToolbar<'a> {
    /// A toolbar titled with the document name.
    pub fn new(metrics: MobileMetrics, language: Language, title: &'a str) -> Self {
        Self {
            metrics,
            language,
            title,
            modified: false,
            can_undo: false,
            can_redo: false,
            can_filter: false,
            filter_count: 0,
            can_export: false,
            can_share: false,
            large: false,
        }
    }

    /// Show the unsaved marker after the title.
    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    /// Enable undo and redo.
    pub fn history(mut self, can_undo: bool, can_redo: bool) -> Self {
        self.can_undo = can_undo;
        self.can_redo = can_redo;
        self
    }

    /// Enable the filter button and show its criteria count.
    pub fn filter(mut self, can_filter: bool, filter_count: usize) -> Self {
        self.can_filter = can_filter;
        self.filter_count = filter_count;
        self
    }

    /// Enable export.
    pub fn export(mut self, can_export: bool) -> Self {
        self.can_export = can_export;
        self
    }

    /// Offer Share in the overflow menu for a file-backed document.
    pub fn share(mut self, can_share: bool) -> Self {
        self.can_share = can_share;
        self
    }

    /// Offer Save As in the overflow menu for a large document.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Draw the toolbar and report what was pressed.
    pub fn show(self, ui: &mut Ui) -> MobileToolbarResponse {
        let mut response = MobileToolbarResponse::default();
        let theme = current(ui.ctx());
        let colors = theme.colors;
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(width, self.metrics.toolbar_height),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, bar_fill(ui));
        hairline(ui, rect, rect.bottom());

        let cell = self.metrics.control_height;
        let inset = 4.0;
        let mut leading = rect.left() + inset;
        let mut leading_cell = |ui: &Ui, icon: Icon, name: &str, enabled: bool, salt: &str| {
            let cell_rect = Rect::from_min_size(
                pos2(leading, rect.top()),
                Vec2::new(cell, self.metrics.toolbar_height),
            );
            leading += cell;
            TouchCell::icon(icon, name)
                .enabled(enabled)
                .show(ui, cell_rect, salt)
        };

        if leading_cell(
            ui,
            Icon::Back,
            tr(self.language, "Documents"),
            true,
            "mobile-toolbar-back",
        )
        .clicked()
        {
            response.back = true;
        }
        if leading_cell(
            ui,
            Icon::Undo,
            tr(self.language, "Undo"),
            self.can_undo,
            "mobile-toolbar-undo",
        )
        .clicked()
        {
            response.undo = true;
        }
        if leading_cell(
            ui,
            Icon::Redo,
            tr(self.language, "Redo"),
            self.can_redo,
            "mobile-toolbar-redo",
        )
        .clicked()
        {
            response.redo = true;
        }

        let trailing_right = rect.right() - inset;
        let mut trailing_left = trailing_right - cell;
        let mut trailing_cell = |ui: &Ui, icon: Icon, name: &str, enabled: bool, salt: &str| {
            let cell_rect = Rect::from_min_size(
                pos2(trailing_left, rect.top()),
                Vec2::new(cell, self.metrics.toolbar_height),
            );
            trailing_left -= cell;
            TouchCell::icon(icon, name)
                .enabled(enabled)
                .show(ui, cell_rect, salt)
        };

        let overflow = trailing_cell(
            ui,
            Icon::Overflow,
            tr(self.language, "More"),
            true,
            "mobile-toolbar-overflow",
        );
        if trailing_cell(
            ui,
            Icon::Export,
            tr(self.language, "Export"),
            self.can_export,
            "mobile-toolbar-export",
        )
        .clicked()
        {
            response.export = true;
        }
        let filter_name = if self.filter_count > 0 {
            format!("{} ({})", tr(self.language, "Filter"), self.filter_count)
        } else {
            tr(self.language, "Filter").to_owned()
        };
        let filter = trailing_cell(
            ui,
            Icon::Filter,
            &filter_name,
            self.can_filter,
            "mobile-toolbar-filter",
        );
        if filter.clicked() {
            response.filter = true;
        }
        if self.filter_count > 0 {
            let badge = pos2(filter.rect.right() - 6.0, filter.rect.top() + 8.0);
            ui.painter().circle_filled(badge, 8.0, colors.accent);
            ui.painter().text(
                badge,
                egui::Align2::CENTER_CENTER,
                self.filter_count.to_string(),
                heading_font(9.0),
                egui::Color32::WHITE,
            );
        }

        let center = Rect::from_min_max(
            pos2(leading + 8.0, rect.top()),
            pos2((trailing_left - 8.0).max(leading + 8.0), rect.bottom()),
        );
        let title = if self.modified {
            format!("{} \u{2022}", self.title)
        } else {
            self.title.to_owned()
        };
        let title_response = ui.interact(
            center,
            ui.scope_id().with("mobile-toolbar-title"),
            egui::Sense::hover(),
        );
        title_response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, title.as_str()));
        let title = cached_elided_title(
            ui.ctx(),
            ui.scope_id(),
            &title,
            center.width(),
            heading_font(self.metrics.title),
            colors.label,
        );
        ui.painter().text(
            center.center(),
            egui::Align2::CENTER_CENTER,
            &title,
            heading_font(self.metrics.title),
            colors.label,
        );

        let mut items = vec![
            OverflowItem::Format,
            OverflowItem::Settings,
            OverflowItem::About,
        ];
        if self.can_share {
            items.push(OverflowItem::Share);
        }
        if self.large {
            items.push(OverflowItem::SaveAs);
        }
        egui::Popup::menu(&overflow).show(|ui| {
            ui.set_min_width(180.0);
            for item in items {
                let label =
                    RichText::new(tr(self.language, item.label_key())).size(self.metrics.body);
                let button =
                    Button::new(label).min_size(Vec2::new(180.0, self.metrics.control_height));
                if ui.add(button).clicked() {
                    response.overflow = Some(item);
                    ui.close();
                }
            }
        });

        response
    }
}

//! The library toolbar: Settings, the title, Add and the overflow menu.

use egui::{Align2, Button, RichText, Role, Sense, Ui, Vec2, WidgetInfo, pos2};
use tabtool_i18n::tr;

use super::super::{LibraryResponse, LibraryView};
use crate::fonts::heading_font;
use crate::icons::Icon;
use crate::mobile::{TouchCell, bar_fill, hairline};
use crate::theme::current;

impl LibraryView {
    pub(super) fn toolbar(&mut self, ui: &mut Ui, response: &mut LibraryResponse) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(width, self.metrics.toolbar_height),
            Sense::hover(),
        );
        ui.painter().rect_filled(rect, 0.0, bar_fill(ui));
        hairline(ui, rect, rect.bottom());

        let cell = self.metrics.control_height;
        let inset = 4.0;
        let settings_rect = egui::Rect::from_min_size(
            pos2(rect.left() + inset, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let settings = TouchCell::icon(Icon::Settings, tr(self.language, "Settings")).show(
            ui,
            settings_rect,
            "library-settings",
        );
        if settings.clicked() {
            response.settings = true;
        }

        let plus_rect = egui::Rect::from_min_size(
            pos2(rect.right() - inset - cell, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let plus = TouchCell::icon(Icon::Plus, tr(self.language, "Add Document")).show(
            ui,
            plus_rect,
            "library-add",
        );
        let overflow_rect = egui::Rect::from_min_size(
            pos2(plus_rect.left() - cell, rect.top()),
            Vec2::new(cell, rect.height()),
        );
        let overflow = TouchCell::icon(Icon::Overflow, tr(self.language, "More")).show(
            ui,
            overflow_rect,
            "library-overflow",
        );

        let center = egui::Rect::from_min_max(
            pos2(settings_rect.right() + 8.0, rect.top()),
            pos2(
                (overflow_rect.left() - 8.0).max(settings_rect.right() + 8.0),
                rect.bottom(),
            ),
        );
        let title = tr(self.language, "Documents");
        let title_response =
            ui.interact(center, ui.scope_id().with("library-title"), Sense::hover());
        title_response.widget_info(|| WidgetInfo::labeled(Role::Label, true, title));
        let theme = current(ui.ctx());
        ui.painter().text(
            center.center(),
            Align2::CENTER_CENTER,
            title,
            heading_font(self.metrics.title),
            theme.colors.label,
        );

        egui::Popup::menu(&plus).show(|ui| {
            ui.set_min_width(180.0);
            if menu_item(ui, self.metrics, tr(self.language, "New Document")) {
                match self.create_document() {
                    Ok(path) => response.open = Some(path),
                    Err(error) => self.notice = Some(error),
                }
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "Import…")) {
                if let Some(path) = self.import_document() {
                    response.open = Some(path);
                }
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "Settings")) {
                response.settings = true;
                ui.close();
            }
        });

        egui::Popup::menu(&overflow).show(|ui| {
            ui.set_min_width(180.0);
            if menu_item(ui, self.metrics, tr(self.language, "Settings")) {
                response.settings = true;
                ui.close();
            }
            if menu_item(ui, self.metrics, tr(self.language, "About")) {
                response.about = true;
                ui.close();
            }
        });
    }
}

fn menu_item(ui: &mut Ui, metrics: crate::mobile::MobileMetrics, label: &str) -> bool {
    ui.add(
        Button::new(RichText::new(label).size(metrics.body))
            .min_size(Vec2::new(180.0, metrics.control_height)),
    )
    .clicked()
}

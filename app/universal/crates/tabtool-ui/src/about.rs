//! The About window content: name, version, description, links and license.
//!
//! The version and the bundled license text come from the desktop crate, so
//! this view stays independent of the filesystem.

use egui::{RichText, ScrollArea, Ui};
use tabtool_i18n::{Language, tr};

use crate::theme::current;
use crate::widgets::{ButtonKind, PushButton};

/// The catalog key of the product name.
pub const NAME_KEY: &str = "TabTool";

/// The short platform-neutral description.
pub const SHORT_DESCRIPTION_KEY: &str = "A simple CSV editor.";

/// The feature description, shown with the platform line.
pub const DESCRIPTION_KEY: &str = "Opens, edits, and saves CSV, TSV, and plain text tables. Detects the encoding and \
     separator, keeps large files responsive, filters and sorts rows, and exports in the \
     format you choose.";

/// The license heading.
pub const LICENSE_KEY: &str = "MIT License";

/// The copyright notice shown above the license text.
pub const COPYRIGHT_KEY: &str = "Free and open source. Copyright © 2026 Dmitry Shurco.";

/// The website shown as a link; a URL, not interface copy.
pub const WEBSITE_LABEL: &str = "tabtool.shurco.app";

/// What the About window asked the app to open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AboutAction {
    /// Open the project website.
    Website,
    /// Open the App Store review page.
    Rate,
}

/// Draws the About content; returns the link the user chose.
pub fn show(ui: &mut Ui, language: Language, version: &str, license: &str) -> Option<AboutAction> {
    let theme = current(ui.ctx());
    let mut action = None;

    egui::Frame::new()
        .fill(theme.colors.window_background)
        .inner_margin(egui::Margin::same(24))
        .show(ui, |ui| {
            ui.set_max_width(400.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr(language, NAME_KEY))
                        .font(crate::fonts::heading_font(theme.typography.title_large))
                        .color(theme.colors.label),
                );
                ui.label(
                    RichText::new(tr(language, "Version %@").replace("%@", version))
                        .size(theme.typography.body)
                        .color(theme.colors.secondary_label),
                );
                ui.add_space(10.0);

                ui.label(tr(language, SHORT_DESCRIPTION_KEY));
                ui.label(tr(language, DESCRIPTION_KEY));

                ui.add_space(12.0);
                if ui.link(WEBSITE_LABEL).clicked() {
                    action = Some(AboutAction::Website);
                }
                ui.add_space(8.0);
                if PushButton::new(tr(language, "Rate TabTool"), ButtonKind::Bordered)
                    .show(ui)
                    .clicked()
                {
                    action = Some(AboutAction::Rate);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr(language, LICENSE_KEY))
                        .font(crate::fonts::heading_font(theme.typography.caption)),
                );
                ui.label(
                    RichText::new(tr(language, COPYRIGHT_KEY))
                        .size(theme.typography.caption)
                        .color(theme.colors.secondary_label),
                );
                ui.add_space(6.0);
                ScrollArea::vertical()
                    .id_salt("about-license")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(license)
                                .monospace()
                                .size(theme.typography.caption)
                                .color(theme.colors.secondary_label),
                        );
                    });
            });
        });

    action
}

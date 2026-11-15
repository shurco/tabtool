//! The About sheet: version, description, links and the license.

use egui::{Label, RichText, Ui};
use tabtool_i18n::tr;

use super::{SheetStyle, touch_wide_button};
use crate::about::{
    AboutAction, COPYRIGHT_KEY, DESCRIPTION_KEY, LICENSE_KEY, NAME_KEY, SHORT_DESCRIPTION_KEY,
    WEBSITE_LABEL,
};
use crate::fonts::heading_font;
use crate::mobile::{SheetOptions, sheet};
use crate::theme::current;
use crate::widgets::ButtonKind;

/// The review page the Rate button opens.
///
/// The public store listing does not exist yet, so the app opens the project
/// website until it does. This is a URL, not interface copy.
pub const RATE_URL: &str = "https://tabtool.shurco.app";

/// What the About sheet reported.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AboutSheetResponse {
    /// The close button asked to dismiss the sheet.
    pub close_requested: bool,
    /// The link the user chose.
    pub action: Option<AboutAction>,
}

/// Draws the About sheet.
pub fn about_sheet(
    ui: &mut Ui,
    style: SheetStyle,
    version: &str,
    license: &str,
) -> AboutSheetResponse {
    let language = style.language;
    let mut action = None;
    let response = sheet(
        ui.ctx(),
        "mobile-about-sheet",
        style.metrics,
        SheetOptions::new(tr(language, "About"), tr(language, "Done"))
            .keyboard_height(style.keyboard_height),
        |ui| {
            let theme = current(ui.ctx());
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr(language, NAME_KEY))
                        .font(heading_font(style.metrics.title))
                        .color(theme.colors.label),
                );
                ui.label(
                    RichText::new(tr(language, "Version %@").replace("%@", version))
                        .size(style.metrics.body)
                        .color(theme.colors.secondary_label),
                );
                ui.add_space(10.0);
                ui.add(
                    Label::new(
                        RichText::new(tr(language, SHORT_DESCRIPTION_KEY))
                            .size(style.metrics.body)
                            .color(theme.colors.label),
                    )
                    .wrap(),
                );
                ui.add(
                    Label::new(
                        RichText::new(tr(language, DESCRIPTION_KEY))
                            .size(style.metrics.caption)
                            .color(theme.colors.secondary_label),
                    )
                    .wrap(),
                );
                ui.add_space(12.0);
                if touch_wide_button(ui, style, WEBSITE_LABEL, ButtonKind::Bordered).clicked() {
                    action = Some(AboutAction::Website);
                }
                ui.add_space(8.0);
                if touch_wide_button(
                    ui,
                    style,
                    tr(language, "Rate TabTool"),
                    ButtonKind::Bordered,
                )
                .clicked()
                {
                    action = Some(AboutAction::Rate);
                }
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tr(language, LICENSE_KEY))
                        .font(heading_font(style.metrics.caption))
                        .color(theme.colors.label),
                );
                ui.add(
                    Label::new(
                        RichText::new(tr(language, COPYRIGHT_KEY))
                            .size(style.metrics.caption)
                            .color(theme.colors.secondary_label),
                    )
                    .wrap(),
                );
                ui.add_space(6.0);
                ui.add(
                    Label::new(
                        RichText::new(license)
                            .monospace()
                            .size(style.metrics.caption)
                            .color(theme.colors.secondary_label),
                    )
                    .wrap(),
                );
            });
        },
    );

    AboutSheetResponse {
        close_requested: response.close_requested,
        action,
    }
}

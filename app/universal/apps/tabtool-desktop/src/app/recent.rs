//! The Open Recent chooser.

use egui::Ui;
use tabtool_i18n::tr;
use tabtool_ui::widgets::{list_row, sheet};

use crate::app::DesktopApp;

impl DesktopApp {
    /// Whether the recent chooser is showing.
    pub fn is_recent_picker_visible(&self) -> bool {
        self.recent_picker
    }

    /// Draws the recent chooser and opens the chosen document.
    pub(super) fn render_recent_documents(&mut self, ui: &mut Ui) {
        if !self.recent_picker {
            return;
        }
        let paths = self.files.recent().to_vec();
        let mut chosen = None;
        let language = self.language;
        let response = sheet(
            ui,
            "recent-documents",
            tr(language, "Open Recent"),
            tr(language, "Cancel"),
            |ui| {
                if paths.is_empty() {
                    ui.label(tr(language, "No Documents"));
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (index, path) in paths.iter().enumerate() {
                            let title = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.display().to_string());
                            let subtitle = path
                                .parent()
                                .map(|parent| parent.display().to_string())
                                .unwrap_or_default();
                            if list_row(ui, &title, &subtitle, false).clicked() {
                                chosen = Some(index);
                            }
                        }
                    });
            },
        );

        if response.close_requested {
            self.recent_picker = false;
        }
        let Some(index) = chosen else {
            return;
        };
        self.recent_picker = false;
        self.open_recent(index);
    }
}

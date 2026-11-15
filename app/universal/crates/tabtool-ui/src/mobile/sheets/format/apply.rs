//! Applying a format choice, with the re-read confirmation it may need.

use egui::Ui;
use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, Storage};
use tabtool_i18n::{Language, tr};

use super::{FormatChoice, FormatSheet};
use crate::document::{
    FormatApplyOutcome, apply_format_configuration, detect_format_configuration,
    request_format_configuration,
};
use crate::mobile::sheets::SheetStyle;
use crate::widgets::{AlertAction, ButtonKind, modal};

impl FormatSheet {
    pub(super) fn apply(
        &mut self,
        choice: FormatChoice,
        document: &mut Document,
        storage: &dyn Storage,
        language: Language,
        prefers_chinese: bool,
    ) {
        match choice {
            FormatChoice::Configuration(configuration) => {
                self.request(document, storage, language, configuration);
            }
            FormatChoice::Detect => {
                match detect_format_configuration(document, storage, language, prefers_chinese) {
                    Ok(detected) => self.request(document, storage, language, detected),
                    Err(alert) => self.alert = Some(alert),
                }
            }
        }
    }

    fn request(
        &mut self,
        document: &mut Document,
        storage: &dyn Storage,
        language: Language,
        configuration: CsvConfiguration,
    ) {
        match request_format_configuration(document, storage, language, configuration) {
            FormatApplyOutcome::Applied => self.alert = None,
            FormatApplyOutcome::NeedsConfirmation(configuration) => {
                self.pending = Some(configuration);
            }
            FormatApplyOutcome::Failed(alert) => self.alert = Some(alert),
        }
    }

    pub(super) fn show_confirmation(
        &mut self,
        ui: &mut Ui,
        style: SheetStyle,
        document: &mut Document,
        storage: &dyn Storage,
    ) {
        let Some(configuration) = self.pending.clone() else {
            return;
        };
        let language = style.language;
        let actions = [
            AlertAction::new(tr(language, "Re-read"), ButtonKind::Destructive),
            AlertAction::new(tr(language, "Cancel"), ButtonKind::Bordered),
        ];
        let pressed = modal(
            ui,
            "mobile-format-reread",
            tr(language, "Re-read the file and discard unsaved edits?"),
            tr(
                language,
                "The table will be replaced with the last saved version from disk. The file itself is not changed.",
            ),
            &actions,
        );
        let Some(index) = pressed else {
            return;
        };
        self.pending = None;
        if index == 0
            && let Some(alert) =
                apply_format_configuration(document, storage, language, &configuration, true)
        {
            self.alert = Some(alert);
        }
    }
}

//! Format configuration changes: apply, re-read and detect.

use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, DocumentError, FormatChange, Storage, plan_format_change};
use tabtool_i18n::{Language, tr};

use crate::document::{ChromeAlert, DocumentView, FormatBarAction, alert_message};

/// The result of asking to apply a format configuration.
pub(crate) enum FormatApplyOutcome {
    /// The document changed.
    Applied,
    /// Unsaved edits would be discarded; the caller must ask.
    NeedsConfirmation(CsvConfiguration),
    /// The change failed and the document kept its table and format.
    Failed(ChromeAlert),
}

/// Applies a candidate configuration through [`Document::apply_parsing_configuration`].
pub(crate) fn apply_format_configuration(
    document: &mut Document,
    storage: &dyn Storage,
    language: Language,
    configuration: &CsvConfiguration,
    force: bool,
) -> Option<ChromeAlert> {
    match document.apply_parsing_configuration(configuration.clone(), force, storage) {
        Ok(()) => None,
        Err(DocumentError::RequiresConfirmation) => Some(ChromeAlert {
            title: "Re-read Interrupted",
            message: tr(
                language,
                "The document changed while it was being re-read, so the new format was not applied. Try again.",
            )
            .to_owned(),
        }),
        Err(error) => Some(ChromeAlert {
            title: "Format Not Applied",
            message: alert_message(language, error),
        }),
    }
}

/// Applies the smallest operation a format edit needs.
///
/// Decimal and header changes transform the table in memory; parsing changes
/// re-read the file, asking first when unsaved edits would be discarded.
pub(crate) fn request_format_configuration(
    document: &mut Document,
    storage: &dyn Storage,
    language: Language,
    configuration: CsvConfiguration,
) -> FormatApplyOutcome {
    let from = document.configuration().clone();
    match plan_format_change(&from, &configuration) {
        FormatChange::None => FormatApplyOutcome::Applied,
        FormatChange::DecimalMark(mark) if document.has_content() => {
            document.set_decimal_mark(&mark);
            FormatApplyOutcome::Applied
        }
        FormatChange::Header(enabled) if document.has_content() => {
            document.set_header_enabled(enabled);
            FormatApplyOutcome::Applied
        }
        _ => {
            let needs_confirmation =
                document.file_path().is_some() && document.has_content() && document.is_dirty();
            if needs_confirmation {
                return FormatApplyOutcome::NeedsConfirmation(configuration);
            }
            match apply_format_configuration(document, storage, language, &configuration, true) {
                Some(alert) => FormatApplyOutcome::Failed(alert),
                None => FormatApplyOutcome::Applied,
            }
        }
    }
}

/// Detects the format from the file on disk without changing the document.
pub(crate) fn detect_format_configuration(
    document: &Document,
    storage: &dyn Storage,
    language: Language,
    prefers_chinese: bool,
) -> Result<CsvConfiguration, ChromeAlert> {
    document
        .detect_format_from_disk(storage, prefers_chinese)
        .map_err(|error| ChromeAlert {
            title: "Format Not Detected",
            message: alert_message(language, error),
        })
}

impl DocumentView<'_> {
    pub(in crate::document) fn apply_format_action(&mut self, action: FormatBarAction) {
        self.commit_active_editor();
        match action {
            FormatBarAction::Configuration(configuration) => {
                self.request_configuration(configuration);
            }
            FormatBarAction::Detect => self.detect_format(),
        }
    }

    pub(in crate::document) fn request_configuration(&mut self, configuration: CsvConfiguration) {
        match request_format_configuration(
            self.document,
            self.storage,
            self.language,
            configuration,
        ) {
            FormatApplyOutcome::Applied => self.ui_state.alert = None,
            FormatApplyOutcome::NeedsConfirmation(configuration) => {
                self.ui_state.pending_reparse = Some(configuration);
            }
            FormatApplyOutcome::Failed(alert) => self.ui_state.alert = Some(alert),
        }
    }

    pub(in crate::document) fn apply_configuration(
        &mut self,
        configuration: &CsvConfiguration,
        force: bool,
    ) {
        match apply_format_configuration(
            self.document,
            self.storage,
            self.language,
            configuration,
            force,
        ) {
            Some(alert) => self.ui_state.alert = Some(alert),
            None => self.ui_state.alert = None,
        }
    }

    pub(in crate::document) fn detect_format(&mut self) {
        if self.document.file_path().is_none() {
            return;
        }
        match detect_format_configuration(
            self.document,
            self.storage,
            self.language,
            self.prefers_chinese,
        ) {
            Ok(detected) => {
                if !self.document.has_content() {
                    self.apply_configuration(&detected, true);
                } else if detected != *self.document.configuration() {
                    self.request_configuration(detected);
                }
            }
            Err(alert) => self.ui_state.alert = Some(alert),
        }
    }
}

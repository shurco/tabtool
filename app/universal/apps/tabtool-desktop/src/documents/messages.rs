//! Localized messages for save state.

use tabtool_core::CsvError;
use tabtool_document::DocumentError;
use tabtool_i18n::{Language, tr};

/// The localized "Saving…" label with a rounded percentage.
pub(crate) fn saving_text(language: Language, fraction: f64) -> String {
    tabtool_ui::document::saving_text(language, fraction)
}

/// The localized explanation for a failed save.
///
/// The banner title says what failed; this is the reason. Every error gets a
/// non-empty sentence so a blocked autosave never shows a title-only banner.
pub(crate) fn issue_message(language: Language, error: DocumentError) -> String {
    match error {
        DocumentError::Csv(CsvError::AmbiguousOutput) => {
            tr(language, "This document can’t be saved with quotes off.").to_owned()
        }
        DocumentError::Csv(CsvError::UnencodableText) => tr(
            language,
            "This document can’t be saved in its current encoding.",
        )
        .to_owned(),
        DocumentError::CannotWriteFile => tr(
            language,
            "Check that the folder exists and that you have permission to save it.",
        )
        .to_owned(),
        DocumentError::NotReady | DocumentError::CannotReadFile => tr(
            language,
            "The document is still loading or could not be read.",
        )
        .to_owned(),
        DocumentError::Busy | DocumentError::RequiresConfirmation => tr(
            language,
            "The document changed while the operation was running. Try again.",
        )
        .to_owned(),
        DocumentError::Csv(_) => tr(language, "The file could not be saved").to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use tabtool_core::CsvError;

    use super::{issue_message, saving_text};

    #[test]
    fn the_german_progress_string_keeps_its_no_break_space() {
        let text = saving_text(tabtool_i18n::Language::De, 0.5);
        assert!(text.contains("50\u{a0}%"), "{text}");
        assert!(!text.contains("%lld"));
        assert!(!text.contains("%%"));
    }

    #[test]
    fn every_save_error_has_an_explanation() {
        for error in [
            tabtool_document::DocumentError::NotReady,
            tabtool_document::DocumentError::RequiresConfirmation,
            tabtool_document::DocumentError::CannotReadFile,
            tabtool_document::DocumentError::CannotWriteFile,
            tabtool_document::DocumentError::Busy,
            tabtool_document::DocumentError::Csv(CsvError::AmbiguousOutput),
            tabtool_document::DocumentError::Csv(CsvError::UnencodableText),
            tabtool_document::DocumentError::Csv(CsvError::CannotDecodeData),
        ] {
            let message = issue_message(tabtool_i18n::Language::En, error);
            assert!(!message.is_empty(), "{error:?} has no explanation");
        }
    }
}

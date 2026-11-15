//! Localized messages for save state.

use tabtool_core::CsvError;
use tabtool_document::DocumentError;
use tabtool_i18n::{Language, tr};
use tabtool_ui::document::error_message;

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
        DocumentError::CannotReadFile => tr(
            language,
            "The document is still loading or could not be read.",
        )
        .to_owned(),
        DocumentError::Csv(_) => tr(language, "The file could not be saved").to_owned(),
        error => error_message(language, error),
    }
}

#[cfg(test)]
mod tests {
    use tabtool_core::CsvError;

    use tabtool_ui::document::saving_text;

    use super::issue_message;

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
            tabtool_document::DocumentError::IndexingNotFinished,
            tabtool_document::DocumentError::SourceChanged,
            tabtool_document::DocumentError::InsufficientDiskSpace,
            tabtool_document::DocumentError::Csv(CsvError::AmbiguousOutput),
            tabtool_document::DocumentError::Csv(CsvError::UnencodableText),
            tabtool_document::DocumentError::Csv(CsvError::CannotDecodeData),
        ] {
            let message = issue_message(tabtool_i18n::Language::En, error);
            assert!(!message.is_empty(), "{error:?} has no explanation");
        }
    }
}

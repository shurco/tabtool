//! Localized messages for save state.

use tabtool_core::CsvError;
use tabtool_document::DocumentError;
use tabtool_i18n::{Language, tr};

/// The localized "Saving…" label with a rounded percentage.
pub(crate) fn saving_text(language: Language, fraction: f64) -> String {
    let percent = (fraction.clamp(0.0, 1.0) * 100.0).round() as i64;
    tr(language, "Saving… %lld%%").replace("%lld%%", &format!("{percent}%"))
}

/// The localized explanation for a failed save.
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
        DocumentError::NotReady | DocumentError::CannotReadFile => {
            tr(language, "The document is not available.").to_owned()
        }
        _ => String::new(),
    }
}

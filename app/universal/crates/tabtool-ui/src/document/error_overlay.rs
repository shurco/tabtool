//! The full-area state shown when the initial read failed, and the user-facing
//! text for read and format failures.

use egui::{Align, Layout, RichText, Ui};
use tabtool_core::CsvError;
use tabtool_document::{DocumentError, DocumentPhase};
use tabtool_i18n::{Language, tr};

use crate::theme::current;
use crate::widgets::{ButtonKind, PushButton};

/// Draws the error overlay; returns whether Detect Automatically was pressed.
pub fn show(ui: &mut Ui, language: Language, phase: &DocumentPhase, can_detect: bool) -> bool {
    let theme = current(ui.ctx());
    let mut detect = false;
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.add_space(theme.metrics.window_padding * 3.0);
        ui.label(
            RichText::new(phase_title(language, phase))
                .font(crate::fonts::heading_font(theme.typography.title_large))
                .color(theme.colors.label),
        );
        ui.add_space(theme.metrics.item_spacing);
        ui.label(
            RichText::new(phase_message(language, phase))
                .size(theme.typography.body)
                .color(theme.colors.label),
        );
        if let Some(hint) = phase_hint(language, phase) {
            ui.add_space(4.0);
            ui.label(
                RichText::new(hint)
                    .size(theme.typography.caption)
                    .color(theme.colors.secondary_label),
            );
        }
        ui.add_space(theme.metrics.item_spacing);
        detect = PushButton::new(tr(language, "Detect Automatically"), ButtonKind::Prominent)
            .enabled(can_detect)
            .show(ui)
            .clicked();
    });
    detect
}

/// The overlay title for a failed phase.
fn phase_title(language: Language, phase: &DocumentPhase) -> &'static str {
    match phase {
        DocumentPhase::ReadFailed(_) => tr(language, "The file could not be read"),
        DocumentPhase::UnreadableFile => tr(language, "The file could not be opened"),
        _ => tr(language, "The document is not available"),
    }
}

/// The overlay message for a failed phase.
fn phase_message(language: Language, phase: &DocumentPhase) -> String {
    match phase {
        DocumentPhase::ReadFailed(error) => csv_message(language, *error),
        DocumentPhase::UnreadableFile => {
            tr(language, "The file could not be read from disk.").to_owned()
        }
        _ => tr(language, "The document is not available.").to_owned(),
    }
}

/// The recovery hint under the overlay message.
fn phase_hint(language: Language, phase: &DocumentPhase) -> Option<&'static str> {
    match phase {
        DocumentPhase::ReadFailed(error) => csv_hint(language, *error),
        DocumentPhase::UnreadableFile => Some(tr(
            language,
            "Check that the file still exists and that you have permission to open it.",
        )),
        _ => None,
    }
}

/// A single alert body: the failure message plus its recovery hint.
pub fn alert_message(language: Language, error: DocumentError) -> String {
    let hint = error_hint(language, error).map(|hint| format!("\n\n{hint}"));
    format!(
        "{}{}",
        error_message(language, error),
        hint.unwrap_or_default()
    )
}

/// The message for a document error.
pub(crate) fn error_message(language: Language, error: DocumentError) -> String {
    match error {
        DocumentError::NotReady => tr(
            language,
            "The document is still loading or could not be read.",
        )
        .to_owned(),
        DocumentError::RequiresConfirmation | DocumentError::Busy => tr(
            language,
            "The document changed while the operation was running. Try again.",
        )
        .to_owned(),
        DocumentError::CannotReadFile => {
            tr(language, "The file could not be read from disk.").to_owned()
        }
        DocumentError::CannotWriteFile => tr(
            language,
            "Check that the folder exists and that you have permission to save it.",
        )
        .to_owned(),
        DocumentError::IndexingNotFinished => tr(
            language,
            "Indexing is not finished. Save again when it completes.",
        )
        .to_owned(),
        DocumentError::SourceChanged => tr(
            language,
            "The file changed on disk since it was opened. Reload it before saving.",
        )
        .to_owned(),
        DocumentError::InsufficientDiskSpace => tr(
            language,
            "There is not enough free disk space to save this file.",
        )
        .to_owned(),
        DocumentError::Csv(error) => csv_message(language, error),
    }
}

/// The recovery hint for a document error.
fn error_hint(language: Language, error: DocumentError) -> Option<&'static str> {
    match error {
        DocumentError::CannotReadFile => Some(tr(
            language,
            "Check that the file still exists and that you have permission to open it.",
        )),
        DocumentError::Csv(error) => csv_hint(language, error),
        DocumentError::NotReady
        | DocumentError::RequiresConfirmation
        | DocumentError::CannotWriteFile
        | DocumentError::Busy => None,
        DocumentError::IndexingNotFinished => Some(tr(
            language,
            "Wait for indexing to finish, or reload the file if it changed on disk.",
        )),
        DocumentError::SourceChanged => Some(tr(
            language,
            "The file changed on disk. Reload to continue.",
        )),
        DocumentError::InsufficientDiskSpace => Some(tr(
            language,
            "Saving a large file needs free space of about twice its size.",
        )),
    }
}

/// The catalog message for a typed CSV error.
fn csv_message(language: Language, error: CsvError) -> String {
    let key = match error {
        CsvError::CannotDecodeData => "The file is not valid text in the selected encoding.",
        CsvError::MalformedQuotedField => {
            "A quoted field is not closed or ends with an incomplete escape."
        }
        CsvError::QuoteInUnquotedField => "A quote character appears inside an unquoted field.",
        CsvError::UnencodableText => "The text cannot be represented in the selected encoding.",
        CsvError::AmbiguousOutput => {
            "A cell contains the separator or a line break, but quotes are off."
        }
        CsvError::UnsupportedLargeFileEncoding => {
            "This encoding cannot be opened in large file mode."
        }
        CsvError::UnsupportedLargeFileDelimiter => {
            "Large file mode needs a single-character ASCII separator, quote and escape."
        }
        CsvError::RowNotEditable => "This row cannot be edited right now.",
    };
    tr(language, key).to_owned()
}

/// The recovery hint for a typed CSV error.
fn csv_hint(language: Language, error: CsvError) -> Option<&'static str> {
    let key = match error {
        CsvError::CannotDecodeData => {
            "Choose another encoding, or detect the format automatically."
        }
        CsvError::MalformedQuotedField | CsvError::QuoteInUnquotedField => {
            "Check the quote style and escape character, or detect the format automatically."
        }
        CsvError::UnencodableText => "Choose an encoding that can represent the text.",
        CsvError::AmbiguousOutput => {
            "Turn quotes on, or remove separators and line breaks from cells."
        }
        CsvError::UnsupportedLargeFileEncoding => "Convert the file to UTF-8 and open it again.",
        CsvError::UnsupportedLargeFileDelimiter => {
            "Choose a separator, quote and escape that are single ASCII characters."
        }
        CsvError::RowNotEditable => {
            "Wait for indexing to finish, or reload the file if it changed on disk."
        }
    };
    Some(tr(language, key))
}

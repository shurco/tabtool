//! Saving a document through the injected storage and dialogs.

use tabtool_document::{Document, DocumentError, Storage};
use tabtool_platform::FileDialogs;

use super::FilePolicy;

/// Which save command started the write.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveMode {
    /// File > Save: in place when the document has a path.
    Save,
    /// File > Save As: always show the save panel.
    SaveAs,
}

/// How a save request ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveOutcome {
    /// The bytes are on disk and the document is clean.
    Saved,
    /// A large streaming save started; poll it through the document.
    LargeStarted,
    /// The user cancelled the save panel.
    Cancelled,
    /// The save failed; the document stays dirty and keeps its path.
    Failed(DocumentError),
}

impl FilePolicy {
    /// Saves the document in place, or through the panel when it has no path.
    pub(crate) fn save(&self, document: &mut Document, suggested_name: &str) -> SaveOutcome {
        self.save_with_mode(document, suggested_name, SaveMode::Save)
    }

    /// Saves the document through the save panel.
    pub(crate) fn save_as(&self, document: &mut Document, suggested_name: &str) -> SaveOutcome {
        self.save_with_mode(document, suggested_name, SaveMode::SaveAs)
    }

    /// Starts the requested save.
    pub(crate) fn save_with_mode(
        &self,
        document: &mut Document,
        suggested_name: &str,
        mode: SaveMode,
    ) -> SaveOutcome {
        save_document(
            document,
            &*self.storage,
            &*self.dialogs,
            suggested_name,
            mode,
        )
    }
}

/// Saves `document` in place or through the save panel.
pub fn save_document(
    document: &mut Document,
    storage: &dyn Storage,
    dialogs: &dyn FileDialogs,
    suggested_name: &str,
    mode: SaveMode,
) -> SaveOutcome {
    if mode == SaveMode::SaveAs || document.file_path().is_none() {
        return save_as(document, storage, dialogs, suggested_name);
    }
    if document.is_large_mode() {
        return match document.start_large_save(None) {
            Ok(_) => SaveOutcome::LargeStarted,
            Err(_) => SaveOutcome::Failed(DocumentError::CannotWriteFile),
        };
    }
    match document.save(storage) {
        Ok(()) => SaveOutcome::Saved,
        Err(error) => SaveOutcome::Failed(error),
    }
}

fn save_as(
    document: &mut Document,
    storage: &dyn Storage,
    dialogs: &dyn FileDialogs,
    suggested_name: &str,
) -> SaveOutcome {
    let name = csv_file_name(suggested_name);
    let Some(path) = dialogs.save_file(&name, "csv") else {
        return SaveOutcome::Cancelled;
    };
    if document.is_large_mode() {
        return match document.start_large_save(Some(&path)) {
            Ok(_) => SaveOutcome::LargeStarted,
            Err(_) => SaveOutcome::Failed(DocumentError::CannotWriteFile),
        };
    }
    match document.save_as(&path, storage) {
        Ok(()) => SaveOutcome::Saved,
        Err(error) => SaveOutcome::Failed(error),
    }
}

/// The save panel name with the `.csv` default.
fn csv_file_name(name: &str) -> String {
    if name.to_ascii_lowercase().ends_with(".csv") {
        name.to_owned()
    } else {
        format!("{name}.csv")
    }
}

#[cfg(test)]
mod tests {
    use super::csv_file_name;

    #[test]
    fn the_save_panel_name_gets_a_csv_extension_once() {
        assert_eq!(csv_file_name("Report"), "Report.csv");
        assert_eq!(csv_file_name("report.CSV"), "report.CSV");
    }
}

//! Process-death state, background saving and the close policy.
//!
//! The app stores only the open document and the top sheet in a small JSON
//! file. Documents themselves are already on disk; autosave writes the small
//! file-backed ones 0.8 s after the last change. Large documents never
//! autosave and prompt before an edit can be lost.

use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use tabtool_document::{Autosave, AutosaveOutcome, Document, DocumentError, Storage};
use tabtool_ui::mobile::SheetKind;

use crate::nav::{NavSnapshot, sheet_from_name, sheet_name};

/// The JSON key of the open document.
const DOCUMENT_KEY: &str = "document";
/// The JSON key of the top sheet, kept for older state files.
const SHEET_KEY: &str = "sheet";
/// The JSON key of the full sheet stack.
const SHEETS_KEY: &str = "sheets";
/// The JSON version, so a future format can migrate rather than misread.
const VERSION_KEY: &str = "version";
/// The current JSON version.
const VERSION: u64 = 1;

/// Reads the process-death state, tolerating a missing or stale file.
pub fn load_state(path: Option<&Path>) -> NavSnapshot {
    let Some(path) = path else {
        return NavSnapshot::default();
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return NavSnapshot::default();
    };
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(&text) else {
        return NavSnapshot::default();
    };
    if object.get(VERSION_KEY).and_then(Value::as_u64) != Some(VERSION) {
        return NavSnapshot::default();
    }
    let sheets = match object.get(SHEETS_KEY).and_then(Value::as_array) {
        Some(sheets) => sheets
            .iter()
            .filter_map(Value::as_str)
            .filter_map(sheet_from_name)
            .collect(),
        None => object
            .get(SHEET_KEY)
            .and_then(Value::as_str)
            .and_then(sheet_from_name)
            .into_iter()
            .collect(),
    };
    NavSnapshot {
        document: object
            .get(DOCUMENT_KEY)
            .and_then(Value::as_str)
            .map(PathBuf::from),
        sheets,
    }
}

/// Writes the process-death state atomically enough for a tiny JSON file.
pub fn save_state(path: Option<&Path>, snapshot: &NavSnapshot) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut object = Map::new();
    object.insert(VERSION_KEY.to_owned(), Value::from(VERSION));
    if let Some(document) = &snapshot.document {
        object.insert(
            DOCUMENT_KEY.to_owned(),
            Value::from(document.to_string_lossy().into_owned()),
        );
    }
    if let Some(sheet) = snapshot.sheets.last() {
        object.insert(SHEET_KEY.to_owned(), Value::from(sheet_name(*sheet)));
    }
    let sheets: Vec<Value> = snapshot
        .sheets
        .iter()
        .map(|sheet| Value::from(sheet_name(*sheet)))
        .collect();
    object.insert(SHEETS_KEY.to_owned(), Value::Array(sheets));
    let data = serde_json::to_vec(&Value::Object(object))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.part");
    std::fs::write(&temporary, data)?;
    std::fs::rename(temporary, path)
}

/// Whether a restored path is a supported document inside the private store.
pub fn restorable(documents_dir: &Path, path: &Path) -> bool {
    path.is_file()
        && crate::intents::is_within(documents_dir, path)
        && crate::intents::is_supported(path)
}

/// Saves a dirty file-backed small document immediately.
///
/// This is the background and back path: small saves are synchronous, so the
/// bytes are on disk before the app leaves the foreground. A large document,
/// an untitled one, or a blocked write is left alone and reports why.
pub fn save_for_background(
    document: &mut Document,
    storage: &dyn Storage,
) -> Result<bool, DocumentError> {
    if document.is_large_mode()
        || document.file_path().is_none()
        || !document.is_dirty()
        || !document.can_save()
    {
        return Ok(false);
    }
    document.save(storage)?;
    Ok(true)
}

/// Whether closing `document` needs a Save / Don't Save / Cancel prompt.
///
/// A clean document closes. A large document prompts because only the
/// explicit streaming save can write it. A small document that the write gate
/// refuses prompts because autosave cannot preserve the edits.
pub fn close_needs_prompt(document: &Document) -> bool {
    document.is_dirty() && (document.is_large_mode() || !document.can_save())
}

/// Runs one autosave tick and reports the typed failure, if any.
pub fn tick_autosave(
    autosave: &mut Autosave,
    now: std::time::Instant,
    document: &mut Document,
    storage: &dyn Storage,
) -> Option<DocumentError> {
    match autosave.tick(now, document, storage) {
        AutosaveOutcome::Blocked(error) => Some(error),
        AutosaveOutcome::Idle
        | AutosaveOutcome::Waiting { .. }
        | AutosaveOutcome::Saved
        | AutosaveOutcome::Untitled
        | AutosaveOutcome::Large
        | AutosaveOutcome::Busy => None,
    }
}

/// A sheet the library can restore without an editor.
pub fn library_sheet(sheet: SheetKind) -> bool {
    matches!(sheet, SheetKind::Settings | SheetKind::About)
}

#[cfg(test)]
mod tests {
    use super::{load_state, restorable, save_state};
    use crate::nav::NavSnapshot;
    use std::path::PathBuf;
    use tabtool_ui::mobile::SheetKind;

    #[test]
    fn the_state_round_trips_and_ignores_a_foreign_version() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("app-state.json");
        let snapshot = NavSnapshot {
            document: Some(PathBuf::from("/data/files/documents/a.csv")),
            sheets: vec![SheetKind::Settings, SheetKind::About],
        };

        save_state(Some(&path), &snapshot).expect("the state is written");
        assert_eq!(load_state(Some(&path)), snapshot);

        std::fs::write(&path, br#"{"version":99,"document":"/x.csv"}"#).expect("the fixture");
        assert_eq!(load_state(Some(&path)), NavSnapshot::default());
    }

    #[test]
    fn only_supported_files_inside_the_store_are_restored() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let inside = directory.path().join("a.csv");
        std::fs::write(&inside, b"a\n").expect("the fixture");
        let outside = directory.path().parent().expect("parent").join("b.csv");
        std::fs::write(&outside, b"b\n").expect("the fixture");

        assert!(restorable(directory.path(), &inside));
        assert!(!restorable(directory.path(), &outside));
        assert!(!restorable(
            directory.path(),
            &directory.path().join("missing.csv")
        ));
    }

    #[test]
    fn a_blocked_small_write_prompts_and_a_clean_document_does_not() {
        use tabtool_core::CsvConfiguration;
        use tabtool_document::Document;

        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("a.csv");
        let configuration = CsvConfiguration {
            quote_character: String::new(),
            escape_character: String::new(),
            ..CsvConfiguration::default()
        };
        let mut document = Document::new(configuration.clone());
        document
            .load_from_bytes(b"a,b\n1,2\n", Some(&path), Some(configuration), false)
            .expect("the fixture parses");
        assert!(!super::close_needs_prompt(&document));

        document.set_cell(0, 0, "x,y");
        assert!(document.is_dirty());
        assert!(!document.can_save(), "quotes off makes the write ambiguous");
        assert!(super::close_needs_prompt(&document));
    }
}

//! File workflows with a fake dialog source and real or in-memory storage.

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_desktop::files::{
    FileWorkflow, ReplaceOutcome, RevertOutcome, SaveOutcome, UnsavedChoice, launch_path,
};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_document::{Document, DocumentError, DocumentPhase, StdStorage};
use tabtool_platform::{Settings, load_settings};

use support::{FakeDialogs, MemoryStorage};

fn workflow(
    dialogs: &Arc<FakeDialogs>,
    storage: SharedStorage,
    settings: Settings,
    settings_path: Option<PathBuf>,
) -> FileWorkflow {
    let dialogs_clone = Arc::clone(dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    FileWorkflow::new(shared_dialogs, storage, settings, settings_path)
}

fn std_storage() -> SharedStorage {
    Arc::new(StdStorage)
}

fn write_file(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, bytes).expect("the fixture file is written");
    path
}

fn cell(document: &Document, row: usize, column: usize) -> String {
    document
        .table()
        .expect("the document has a table")
        .cell(row, column)
}

/// Loads `path` with the defaults, so a test is independent of detection.
fn load_defaults(path: &Path) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_path(path, &StdStorage, Some(configuration), false)
        .expect("the fixture loads");
    document
}

#[test]
fn open_dialog_loads_table_files_and_ignores_other_extensions() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let one = write_file(directory.path(), "one.csv", b"a,b\n1,2\n");
    let two = write_file(directory.path(), "two.tsv", b"x\ty\n3\t4\n");
    let image = write_file(directory.path(), "note.png", b"not a table");
    let dialogs = FakeDialogs::scripted_opens([vec![one.clone(), two.clone(), image]]);
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let opened = workflow.open_dialog();

    assert_eq!(opened.len(), 2, "only table extensions are opened");
    assert!(opened.iter().all(|opened| opened.loaded));
    assert_eq!(opened[0].document.file_path(), Some(one.as_path()));
    assert_eq!(opened[1].document.file_path(), Some(two.as_path()));
    assert_eq!(workflow.recent(), [two, one]);
    assert_eq!(dialogs.open_calls(), 1);
}

#[test]
fn open_dropped_accepts_table_files_in_any_order() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let csv = write_file(directory.path(), "drop.csv", b"a,b\n1,2\n");
    let image = write_file(directory.path(), "drop.png", b"not a table");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let opened = workflow.open_dropped(&[image, csv.clone()]);

    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].document.file_path(), Some(csv.as_path()));
    assert_eq!(workflow.recent(), [csv]);
}

#[test]
fn every_documented_table_extension_is_accepted_case_insensitively() {
    for extension in ["csv", "tsv", "txt", "tab", "dat"] {
        let path = PathBuf::from(format!("document.{extension}"));
        assert!(
            tabtool_desktop::files::is_table_file(&path),
            "{extension} must open"
        );
        let upper = PathBuf::from(format!("document.{}", extension.to_ascii_uppercase()));
        assert!(tabtool_desktop::files::is_table_file(&upper));
    }
    assert!(!tabtool_desktop::files::is_table_file(Path::new(
        "document.png"
    )));
}

#[test]
fn the_launch_path_skips_flags_and_uses_the_first_argument() {
    assert_eq!(
        launch_path(["--windowed".to_owned(), "/tmp/a.csv".to_owned()]),
        Some(PathBuf::from("/tmp/a.csv"))
    );
    assert_eq!(
        launch_path(["--smoke".to_owned()]),
        None,
        "a flag is not a document"
    );
}

#[test]
fn the_launch_argument_opens_a_document() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "launch.csv", b"a,b\n1,2\n");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let launch = launch_path(["--windowed".to_owned(), path.to_string_lossy().into_owned()])
        .expect("the launch path is a file");
    let opened = workflow.open_path(&launch);

    assert!(opened.loaded);
    assert_eq!(opened.document.file_path(), Some(path.as_path()));
    assert_eq!(workflow.recent(), [path]);
}

#[test]
fn a_missing_file_shows_the_read_error_and_stays_out_of_recent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let missing = directory.path().join("missing.csv");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let opened = workflow.open_path(&missing);

    assert!(!opened.loaded);
    assert_eq!(opened.document.phase(), &DocumentPhase::UnreadableFile);
    assert_eq!(opened.document.file_path(), Some(missing.as_path()));
    assert!(workflow.recent().is_empty());
}

#[test]
fn a_directory_path_shows_the_read_error_and_stays_out_of_recent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let opened = workflow.open_path(directory.path());

    assert!(!opened.loaded);
    assert_eq!(opened.document.phase(), &DocumentPhase::UnreadableFile);
    assert!(workflow.recent().is_empty());
}

#[test]
fn recent_keeps_ten_newest_and_drops_missing_paths() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let files: Vec<PathBuf> = (0..11)
        .map(|index| {
            write_file(
                directory.path(),
                &format!("recent-{index}.csv"),
                format!("a,b\n{index},x\n").as_bytes(),
            )
        })
        .collect();
    let newest = write_file(directory.path(), "newest.csv", b"a,b\nnew,x\n");
    let mut recent = vec![directory.path().join("missing.csv")];
    recent.extend(files.iter().rev().cloned());
    let settings = Settings {
        recent_documents: recent,
        ..Settings::default()
    };
    let settings_path = directory.path().join("settings.json");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(
        &dialogs,
        std_storage(),
        settings,
        Some(settings_path.clone()),
    );

    let opened = workflow.open_path(&newest);

    assert!(opened.loaded);
    let recent = workflow.recent();
    assert_eq!(recent.len(), 10);
    assert_eq!(recent[0], newest);
    assert!(!recent.contains(&directory.path().join("missing.csv")));

    let persisted = load_settings(&settings_path);
    assert_eq!(persisted.recent_documents, recent);
}

#[test]
fn forgetting_missing_recent_drops_the_entry() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "forgotten.csv", b"a\n1\n");
    let settings = Settings {
        recent_documents: vec![path.clone()],
        ..Settings::default()
    };
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), settings, None);
    std::fs::remove_file(&path).expect("the fixture is removed");

    workflow.forget_missing_recent();

    assert!(workflow.recent().is_empty());
}

#[test]
fn open_recent_drops_a_path_deleted_after_loading() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "recent.csv", b"a,b\n1,2\n");
    let settings = Settings {
        recent_documents: vec![path.clone()],
        ..Settings::default()
    };
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), settings, None);
    std::fs::remove_file(&path).expect("the fixture is removed");

    assert!(workflow.open_recent(0).is_none());
    assert!(workflow.recent().is_empty(), "a missing path is dropped");
}

#[test]
fn open_recent_keeps_the_index_the_list_showed_after_a_deletion() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let first = write_file(directory.path(), "first.csv", b"a\n1\n");
    let second = write_file(directory.path(), "second.csv", b"a\n2\n");
    let third = write_file(directory.path(), "third.csv", b"a\n3\n");
    let settings = Settings {
        recent_documents: vec![first.clone(), second.clone(), third.clone()],
        ..Settings::default()
    };
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), settings, None);
    std::fs::remove_file(&first).expect("the fixture is removed");

    let opened = workflow
        .open_recent(2)
        .expect("the third entry still opens at its displayed index");

    assert_eq!(opened.document.file_path(), Some(third.as_path()));
    assert_eq!(workflow.recent()[..2], [third, second]);
}

#[test]
fn a_new_document_uses_the_new_document_settings_and_is_dirty() {
    let settings = Settings {
        new_document_separator: ";".to_owned(),
        new_document_first_row_as_header: true,
        new_document_encoding: Encoding::Windows1252,
        ..Settings::default()
    };
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, std_storage(), settings, None);

    let document = workflow.new_document();

    assert_eq!(document.configuration().column_separator, ";");
    assert!(document.configuration().first_row_as_header);
    assert_eq!(document.configuration().encoding, Encoding::Windows1252);
    assert!(document.is_dirty());
    assert_eq!(document.file_path(), None);
}

#[test]
fn save_uses_the_current_path_without_a_panel() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "save.csv", b"a,b\n1,2\n");
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = load_defaults(&path);
    document.set_cell(0, 0, "edited");

    let outcome = workflow.save(&mut document, "save.csv");

    assert_eq!(outcome, SaveOutcome::Saved);
    assert!(
        dialogs.saves().is_empty(),
        "an in-place save needs no panel"
    );
    assert!(!document.is_dirty());
    assert_eq!(
        std::fs::read_to_string(&path).expect("the saved file is readable"),
        "\"edited\",\"b\"\n\"1\",\"2\""
    );
}

#[test]
fn save_as_uses_the_panel_and_binds_the_path_after_success() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("Report.csv");
    let dialogs = FakeDialogs::scripted([Some(path.clone())]);
    let workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = workflow.new_document();
    document.set_cell(0, 0, "edited");

    let outcome = workflow.save_as(&mut document, "Report");

    assert_eq!(outcome, SaveOutcome::Saved);
    assert_eq!(
        dialogs.saves(),
        [("Report.csv".to_owned(), "csv".to_owned())]
    );
    assert_eq!(document.file_path(), Some(path.as_path()));
    assert!(!document.is_dirty());
}

#[test]
fn a_failed_save_as_keeps_the_document_dirty_and_untitled() {
    let storage = Arc::new(MemoryStorage::new(b""));
    let dialogs = FakeDialogs::scripted([Some(PathBuf::from("/virtual/report.csv"))]);
    let shared: SharedStorage = storage.clone();
    let workflow = workflow(&dialogs, shared, Settings::default(), None);
    let mut document = workflow.new_document();
    document.set_cell(0, 0, "edited");
    storage.fail_writes();

    let outcome = workflow.save_as(&mut document, "Report");

    assert_eq!(outcome, SaveOutcome::Failed(DocumentError::CannotWriteFile));
    assert_eq!(document.file_path(), None);
    assert!(document.is_dirty());
}

#[test]
fn cancelling_the_save_panel_keeps_the_document_dirty() {
    let storage = Arc::new(MemoryStorage::new(b""));
    let dialogs = FakeDialogs::cancelled();
    let shared: SharedStorage = storage.clone();
    let workflow = workflow(&dialogs, shared, Settings::default(), None);
    let mut document = workflow.new_document();
    document.set_cell(0, 0, "edited");

    let outcome = workflow.save_as(&mut document, "Report");

    assert_eq!(outcome, SaveOutcome::Cancelled);
    assert_eq!(document.file_path(), None);
    assert!(document.is_dirty());
    assert_eq!(storage.writes(), 0);
}

#[test]
fn a_large_document_save_starts_the_streaming_job() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "large.csv", b"a,b\n1,2\n");
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(&path, None, false, 0)
        .expect("open the fixture large");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
    document
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let outcome = workflow.save(&mut document, "large.csv");

    assert_eq!(outcome, SaveOutcome::LargeStarted);
    assert!(
        matches!(
            document.large_status(),
            tabtool_document::LargeStatus::Saving(_)
        ),
        "the save must use the streaming job"
    );
    document.cancel_large_save();
    let _ = document.wait_for_large_save();
}

#[test]
fn revert_asks_when_dirty_and_restores_one_undo_step() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "revert.csv", b"a,b\n1,2\n");
    let before = std::fs::read(&path).expect("the fixture is readable");
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = load_defaults(&path);
    document.set_cell(0, 0, "edited");

    assert_eq!(
        workflow.revert(&mut document, false),
        RevertOutcome::NeedsConfirmation
    );
    assert_eq!(cell(&document, 0, 0), "edited");

    assert_eq!(
        workflow.revert(&mut document, true),
        RevertOutcome::Reverted
    );
    assert_eq!(cell(&document, 0, 0), "a");
    assert!(!document.is_dirty());
    assert_eq!(std::fs::read(&path).expect("the file is unchanged"), before);
    assert!(document.can_undo(), "revert is one undo action");

    document.undo();
    assert_eq!(cell(&document, 0, 0), "edited");
}

#[test]
fn revert_is_unavailable_for_an_untitled_document() {
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = workflow.new_document();

    assert_eq!(
        workflow.revert(&mut document, true),
        RevertOutcome::Unavailable
    );
}

#[test]
fn an_export_session_uses_the_document_name_and_configuration() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let shared: SharedStorage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let mut document = Document::new(CsvConfiguration::default());
    document
        .load_from_path(
            Path::new("/virtual/Sales.csv"),
            &storage,
            Some(CsvConfiguration::default()),
            false,
        )
        .expect("load the fixture");
    let dialogs = FakeDialogs::cancelled();
    let workflow = workflow(&dialogs, shared, Settings::default(), None);

    let session = workflow
        .export_session(&document)
        .expect("an export session");

    assert_eq!(session.base_name, "Sales-export");
    assert_eq!(session.suggested_name(), "Sales-export.csv");
    assert_eq!(session.file_extension(), "csv");
    assert_eq!(session.options.configuration, *document.configuration());
}

#[test]
fn a_tab_export_suggests_tsv_and_writes_tsv() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let document_path = write_file(directory.path(), "Sales.csv", b"a,b\n1,2\n");
    let export_path = directory.path().join("Sales-export.tsv");
    let dialogs = FakeDialogs::scripted([Some(export_path.clone())]);
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let document = workflow.open_path(&document_path).document;
    let mut session = workflow
        .export_session(&document)
        .expect("an export session");
    session.options.configuration.column_separator = "\t".to_owned();

    let outcome = workflow.export(&session, false);

    assert_eq!(outcome, tabtool_desktop::files::ExportOutcome::Saved);
    assert_eq!(
        dialogs.saves(),
        [("Sales-export.tsv".to_owned(), "tsv".to_owned())]
    );
    assert_eq!(
        std::fs::read_to_string(&export_path).expect("the export is readable"),
        "\"a\"\t\"b\"\n\"1\"\t\"2\""
    );
}

#[test]
fn an_ambiguous_export_asks_and_writes_only_after_confirmation() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let document_path = write_file(directory.path(), "ambiguous.csv", b"1.5\n");
    let export_path = directory.path().join("ambiguous-export.csv");
    let dialogs = FakeDialogs::scripted([Some(export_path.clone())]);
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let document = workflow.open_path(&document_path).document;
    let mut session = workflow
        .export_session(&document)
        .expect("an export session");
    session.options.configuration.quote_character.clear();
    session.options.configuration.escape_character.clear();
    session.options.configuration.decimal_mark = ",".to_owned();

    let outcome = workflow.export(&session, false);

    assert_eq!(
        outcome,
        tabtool_desktop::files::ExportOutcome::NeedsConfirmation
    );
    assert!(
        dialogs.saves().is_empty(),
        "no path is chosen before asking"
    );
    assert!(!export_path.exists(), "nothing is written before asking");
    assert_eq!(cell(&document, 0, 0), "1.5");
    assert_eq!(document.configuration().decimal_mark, ".");

    let outcome = workflow.export(&session, true);

    assert_eq!(outcome, tabtool_desktop::files::ExportOutcome::Saved);
    assert_eq!(
        std::fs::read_to_string(&export_path).expect("the export is readable"),
        "1,5"
    );
}

#[test]
fn an_unencodable_export_is_refused_without_a_panel_or_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let document_path = write_file(directory.path(), "cyrillic.csv", "ж\n".as_bytes());
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let document = workflow.open_path(&document_path).document;
    let mut session = workflow
        .export_session(&document)
        .expect("an export session");
    session.options.configuration.encoding = Encoding::Windows1252;

    let outcome = workflow.export(&session, false);

    assert_eq!(
        outcome,
        tabtool_desktop::files::ExportOutcome::BlockedUnencodable
    );
    assert!(dialogs.saves().is_empty());
    assert_eq!(
        document.table().expect("table").headers(),
        Some(vec!["ж".to_owned()]),
        "the detected header is the unencodable text"
    );
}

#[test]
fn cancelling_the_export_panel_writes_nothing() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let document_path = write_file(directory.path(), "cancel.csv", b"a,b\n1,2\n");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let document = workflow.open_path(&document_path).document;
    let session = workflow
        .export_session(&document)
        .expect("an export session");

    let outcome = workflow.export(&session, false);

    assert_eq!(outcome, tabtool_desktop::files::ExportOutcome::Cancelled);
    assert_eq!(dialogs.saves().len(), 1);
}

#[test]
fn a_failed_export_write_keeps_the_document_unchanged() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let shared: SharedStorage = storage.clone();
    let mut document = Document::new(CsvConfiguration::default());
    document
        .load_from_path(
            Path::new("/virtual/failed.csv"),
            &*storage,
            Some(CsvConfiguration::default()),
            false,
        )
        .expect("load the fixture");
    let dialogs = FakeDialogs::scripted([Some(PathBuf::from("/virtual/out.csv"))]);
    let workflow = workflow(&dialogs, shared, Settings::default(), None);
    let session = workflow
        .export_session(&document)
        .expect("an export session");
    storage.fail_writes();
    let before = cell(&document, 0, 0);

    let outcome = workflow.export(&session, false);

    assert_eq!(
        outcome,
        tabtool_desktop::files::ExportOutcome::Failed(DocumentError::CannotWriteFile)
    );
    assert_eq!(cell(&document, 0, 0), before);
    assert_eq!(*document.configuration(), CsvConfiguration::default());
}

#[test]
fn replace_with_prompts_for_a_dirty_document_and_cancel_keeps_it() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"x,y\n3,4\n");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = load_defaults(&old);
    document.set_cell(0, 0, "edited");

    assert_eq!(
        workflow.replace_with(&mut document, &new, None, "old.csv"),
        ReplaceOutcome::NeedsConfirmation
    );
    assert_eq!(document.file_path(), Some(old.as_path()));

    assert_eq!(
        workflow.replace_with(&mut document, &new, Some(UnsavedChoice::Cancel), "old.csv"),
        ReplaceOutcome::Cancelled
    );
    assert_eq!(document.file_path(), Some(old.as_path()));
    assert_eq!(cell(&document, 0, 0), "edited");
}

#[test]
fn replace_with_dont_save_opens_the_new_file_without_writing() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"x,y\n3,4\n");
    let before = std::fs::read(&old).expect("the fixture is readable");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = load_defaults(&old);
    document.set_cell(0, 0, "edited");

    let ReplaceOutcome::Opened(opened) = workflow.replace_with(
        &mut document,
        &new,
        Some(UnsavedChoice::DontSave),
        "old.csv",
    ) else {
        panic!("the new file must open");
    };

    assert!(opened.loaded);
    assert_eq!(opened.document.file_path(), Some(new.as_path()));
    assert_eq!(
        opened.document.table().expect("table").headers(),
        Some(vec!["x".to_owned(), "y".to_owned()])
    );
    assert_eq!(cell(&opened.document, 0, 0), "3");
    assert_eq!(
        std::fs::read(&old).expect("the old file is unchanged"),
        before
    );
}

#[test]
fn replace_with_save_writes_the_old_document_then_opens_the_new_one() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"x,y\n3,4\n");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);
    let mut document = load_defaults(&old);
    document.set_cell(0, 0, "edited");

    let ReplaceOutcome::Opened(opened) =
        workflow.replace_with(&mut document, &new, Some(UnsavedChoice::Save), "old.csv")
    else {
        panic!("the new file must open after saving");
    };

    assert_eq!(
        std::fs::read_to_string(&old).expect("the old file is readable"),
        "\"edited\",\"b\"\n\"1\",\"2\""
    );
    assert_eq!(opened.document.file_path(), Some(new.as_path()));
    assert_eq!(
        opened.document.table().expect("table").headers(),
        Some(vec!["x".to_owned(), "y".to_owned()])
    );
    assert_eq!(cell(&opened.document, 0, 0), "3");
}

#[test]
fn replace_with_a_failed_save_does_not_open_the_new_file() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let shared: SharedStorage = storage.clone();
    let mut document = Document::new(CsvConfiguration::default());
    document
        .load_from_path(
            Path::new("/virtual/old.csv"),
            &*storage,
            Some(CsvConfiguration::default()),
            false,
        )
        .expect("load the fixture");
    document.set_cell(0, 0, "edited");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, shared, Settings::default(), None);
    storage.fail_writes();

    let outcome = workflow.replace_with(
        &mut document,
        Path::new("/virtual/new.csv"),
        Some(UnsavedChoice::Save),
        "old.csv",
    );

    assert_eq!(
        outcome,
        ReplaceOutcome::Failed(DocumentError::CannotWriteFile)
    );
    assert_eq!(document.file_path(), Some(Path::new("/virtual/old.csv")));
}

#[test]
fn replace_with_save_starts_the_large_save_and_reports_busy() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old-large.csv", b"a,b\n1,2\n3,4\n");
    let new = write_file(directory.path(), "new.csv", b"x,y\n3,4\n");
    let mut document = support::large_ready(&old);
    document
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let outcome = workflow.replace_with(
        &mut document,
        &new,
        Some(UnsavedChoice::Save),
        "old-large.csv",
    );

    assert_eq!(outcome, ReplaceOutcome::Busy);
    assert!(matches!(
        document.large_status(),
        tabtool_document::LargeStatus::Saving(_)
    ));
    document.cancel_large_save();
    let _ = document.wait_for_large_save();
}

#[test]
fn replace_with_a_saving_large_document_reports_busy() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old-large.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"x,y\n3,4\n");
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(&old, None, false, 0)
        .expect("open the fixture large");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
    document
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    document.start_large_save(None).expect("start the save");
    let dialogs = FakeDialogs::cancelled();
    let mut workflow = workflow(&dialogs, std_storage(), Settings::default(), None);

    let outcome = workflow.replace_with(&mut document, &new, None, "old-large.csv");

    assert_eq!(outcome, ReplaceOutcome::Busy);
    assert_eq!(document.file_path(), Some(old.as_path()));
    document.cancel_large_save();
    let _ = document.wait_for_large_save();
}

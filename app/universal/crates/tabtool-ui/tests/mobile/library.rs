//! The Documents library: listing, naming, creation, import, rename, delete
//! and the touch screen that composes them.

use core::cell::{Cell, RefCell};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, UNIX_EPOCH};

use tabtool_core::{CsvConfiguration, CsvParser, Locale};
use tabtool_i18n::{ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE, Language, LocalizedError, tr};
use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{
    DocumentFiles, ExportOutcome, LibraryClock, LibraryResponse, LibrarySort, LibraryView,
    MobileMetrics, TouchTextInputOutcome, available_name, format_modified, format_size,
    new_document_name,
};
use tempfile::TempDir;

use crate::support::{FakeTouchInput, en_locale};

const EN: Language = Language::En;

fn de_locale() -> Locale {
    Locale::try_from_str("de-DE").expect("de-DE is a valid locale")
}

fn gb_locale() -> Locale {
    Locale::try_from_str("en-GB").expect("en-GB is a valid locale")
}

fn fr_locale() -> Locale {
    Locale::try_from_str("fr-FR").expect("fr-FR is a valid locale")
}

/// A documents folder on disk with a one-shot import picker and one queued
/// platform failure.
struct TestFiles {
    directory: PathBuf,
    picker: RefCell<Option<PathBuf>>,
    extra: RefCell<Vec<PathBuf>>,
    in_progress: Cell<bool>,
    create_calls: Cell<usize>,
    fail_delete: Cell<bool>,
    fail_rename: Cell<bool>,
    pending_error: RefCell<Option<LocalizedError>>,
}

impl TestFiles {
    fn new() -> (TempDir, Rc<Self>) {
        let directory = TempDir::new().expect("a temporary documents folder");
        let path = directory.path().to_path_buf();
        (directory, Self::at(path))
    }

    fn at(directory: PathBuf) -> Rc<Self> {
        Rc::new(Self {
            directory,
            picker: RefCell::new(None),
            extra: RefCell::new(Vec::new()),
            in_progress: Cell::new(false),
            create_calls: Cell::new(0),
            fail_delete: Cell::new(false),
            fail_rename: Cell::new(false),
            pending_error: RefCell::new(None),
        })
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.directory.join(name);
        std::fs::write(&path, contents).expect("the fixture file is written");
        path
    }
}

impl DocumentFiles for TestFiles {
    fn documents_dir(&self) -> PathBuf {
        self.directory.clone()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        let mut documents: Vec<PathBuf> = std::fs::read_dir(&self.directory)
            .expect("the documents folder is readable")
            .map(|entry| entry.expect("a document entry").path())
            .filter(|path| path.is_file() && supported(path))
            .collect();
        documents.extend(self.extra.borrow().iter().cloned());
        documents
    }

    fn existing_names(&self) -> Vec<String> {
        std::fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }

    fn create_document(&self, name: &str, data: &[u8]) -> std::io::Result<PathBuf> {
        self.create_calls.set(self.create_calls.get() + 1);
        let path = self.directory.join(name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(data)?;
        Ok(path)
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        if self.in_progress.get() {
            return None;
        }
        let source = self.picker.borrow_mut().take()?;
        let name = source.file_name()?;
        let destination = self.directory.join(name);
        std::fs::copy(source, &destination).ok()?;
        Some(destination)
    }

    fn import_in_progress(&self) -> bool {
        self.in_progress.get()
    }

    fn export_copy(&self, _source: &Path, _suggested_name: &str) -> ExportOutcome {
        ExportOutcome::Cancelled
    }

    fn delete(&self, path: &Path) -> std::io::Result<()> {
        if self.fail_delete.get() {
            return Err(std::io::Error::other("delete failed"));
        }
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        if self.fail_rename.get() {
            return Err(std::io::Error::other("rename failed"));
        }
        std::fs::rename(from, to)
    }

    fn take_error(&self) -> Option<LocalizedError> {
        self.pending_error.borrow_mut().take()
    }
}

fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["csv", "tsv", "txt", "tab", "dat"]
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

fn library(files: &Rc<TestFiles>) -> LibraryView {
    library_with_clock(files, LibraryClock::default())
}

fn library_with_clock(files: &Rc<TestFiles>, clock: LibraryClock) -> LibraryView {
    LibraryView::new(
        files.clone(),
        en_locale(),
        clock,
        MobileMetrics::phone(),
        EN,
    )
}

fn names(view: &LibraryView) -> Vec<String> {
    view.documents()
        .iter()
        .map(|document| document.name().to_owned())
        .collect()
}

fn set_modified(path: &Path, seconds: u64) {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("the fixture is writable");
    let modified = UNIX_EPOCH + Duration::from_secs(seconds);
    file.set_times(std::fs::FileTimes::new().set_modified(modified))
        .expect("the modification date is set");
}

#[test]
fn every_library_error_key_exists_in_the_catalog() {
    for key in [ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE] {
        assert!(
            tabtool_i18n::keys().any(|candidate| candidate == key),
            "{key} is missing from the catalog"
        );
    }
}

// -- Naming ---------------------------------------------------------------

#[test]
fn the_first_new_document_has_the_plain_name() {
    assert_eq!(
        new_document_name(std::iter::empty::<&str>(), "csv"),
        "Untitled.csv"
    );
}

#[test]
fn new_documents_count_up() {
    assert_eq!(new_document_name(["untitled.csv"], "csv"), "Untitled 2.csv");
    assert_eq!(
        new_document_name(["untitled.csv", "untitled 2.csv"], "csv"),
        "Untitled 3.csv"
    );
}

#[test]
fn new_document_names_ignore_case() {
    assert_eq!(new_document_name(["UNTITLED.CSV"], "csv"), "Untitled 2.csv");
}

#[test]
fn an_import_keeps_its_name_when_free() {
    assert_eq!(available_name("data.csv", ["other.csv"]), "data.csv");
}

#[test]
fn an_import_gets_a_number_when_the_name_is_taken() {
    assert_eq!(available_name("data.csv", ["data.csv"]), "data 2.csv");
    assert_eq!(
        available_name("data.csv", ["data.csv", "data 2.csv"]),
        "data 3.csv"
    );
}

#[test]
fn a_name_without_an_extension_keeps_its_shape() {
    assert_eq!(available_name("notes", ["notes"]), "notes 2");
}

// -- Listing --------------------------------------------------------------

#[test]
fn lists_only_supported_files_newest_first() {
    let (_directory, files) = TestFiles::new();
    let old = files.write("old.csv", "a,b\n");
    files.write("new.tsv", "a\tb\n");
    files.write("notes.md", "not a table\n");
    set_modified(&old, 100);
    let mut view = library(&files);

    view.refresh();

    assert_eq!(names(&view), ["new.tsv", "old.csv"]);
    assert_eq!(view.sort(), LibrarySort::NewestFirst);
}

#[test]
fn documents_with_the_same_date_sort_by_name() {
    let (_directory, files) = TestFiles::new();
    let beta = files.write("beta.csv", "a\n");
    let alpha = files.write("alpha.csv", "a\n");
    set_modified(&beta, 500);
    set_modified(&alpha, 500);
    let mut view = library(&files);

    view.refresh();

    assert_eq!(names(&view), ["alpha.csv", "beta.csv"]);
}

#[test]
fn a_listed_file_that_is_gone_is_skipped() {
    let (_directory, files) = TestFiles::new();
    files.write("here.csv", "a\n");
    files
        .extra
        .borrow_mut()
        .push(files.documents_dir().join("gone.csv"));
    let mut view = library(&files);

    view.refresh();

    assert_eq!(names(&view), ["here.csv"]);
}

#[test]
fn hidden_files_are_not_listed() {
    let (_directory, files) = TestFiles::new();
    files.write(".notes.csv", "a\n");
    files.write("notes.csv", "a\n");
    let mut view = library(&files);

    view.refresh();

    assert_eq!(names(&view), ["notes.csv"]);
}

#[test]
fn sizes_and_dates_come_from_the_file_metadata() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("notes.csv", "a,b\n1,2\n");
    set_modified(&path, 1_700_000_000);
    let mut view = library(&files);

    view.refresh();

    let document = &view.documents()[0];
    assert_eq!(document.size(), 8);
    assert_eq!(
        document.modified(),
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    );
}

#[test]
fn sizes_use_the_locale_decimal_separator() {
    assert_eq!(format_size(0, &en_locale()), "0 B");
    assert_eq!(format_size(1500, &en_locale()), "1.5 KB");
    assert_eq!(format_size(1500, &de_locale()), "1,5 KB");
    assert_eq!(format_size(2_000_000, &en_locale()), "2 MB");
    assert_eq!(format_size(999_999, &en_locale()), "1 MB");
}

#[test]
fn dates_use_the_locale_short_order() {
    let modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let clock = LibraryClock::default();

    assert_eq!(
        format_modified(modified, &en_locale(), clock),
        "11/14/2023, 22:13"
    );
    assert_eq!(
        format_modified(modified, &de_locale(), clock),
        "14.11.2023, 22:13"
    );
    assert_eq!(
        format_modified(modified, &gb_locale(), clock),
        "14/11/2023, 22:13"
    );
    assert_eq!(
        format_modified(modified, &fr_locale(), clock),
        "14/11/2023, 22:13"
    );
}

#[test]
fn dates_use_the_platform_clock() {
    let noon = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    // 22:13 UTC is 14:13 the same day at UTC−8.
    let pacific = LibraryClock::new(-480, true);
    assert_eq!(
        format_modified(noon, &en_locale(), pacific),
        "11/14/2023, 2:13 PM"
    );
    // 06:13 UTC is 22:13 the previous day at UTC−8.
    let before_midnight = UNIX_EPOCH + Duration::from_secs(1_699_942_400);
    assert_eq!(
        format_modified(before_midnight, &en_locale(), pacific),
        "11/13/2023, 10:13 PM"
    );
    // The same instant is the next hour in Berlin on a 24-hour clock.
    let berlin = LibraryClock::new(60, false);
    assert_eq!(
        format_modified(noon, &de_locale(), berlin),
        "14.11.2023, 23:13"
    );
}

// -- Create, import, rename, delete ---------------------------------------

#[test]
fn creates_a_blank_document_and_lists_it() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);

    let path = view.create_document().expect("the document is created");

    assert_eq!(path.file_name().expect("a file name"), "Untitled.csv");
    assert!(path.is_file());
    assert_eq!(files.create_calls.get(), 1, "the platform creates the file");
    assert_eq!(names(&view), ["Untitled.csv"]);

    let bytes = std::fs::read(&path).expect("the new document is readable");
    let mut parser = CsvParser::from_bytes(&bytes, &CsvConfiguration::default())
        .expect("the new document is valid");
    let rows = parser.read_all().expect("the new document parses");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 3);
}

#[test]
fn creates_use_the_configured_new_document_format() {
    let (_directory, files) = TestFiles::new();
    let configuration = CsvConfiguration {
        column_separator: "\t".to_owned(),
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut view = library(&files).new_document_configuration(configuration);

    let path = view.create_document().expect("the document is created");

    assert_eq!(
        std::fs::read(&path).expect("the new document is readable"),
        b"\t\t"
    );
}

#[test]
fn a_second_new_document_counts_up() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);

    view.create_document().expect("the first document");
    let second = view.create_document().expect("the second document");

    assert_eq!(second.file_name().expect("a file name"), "Untitled 2.csv");
    assert_eq!(names(&view), ["Untitled 2.csv", "Untitled.csv"]);
}

#[test]
fn a_new_document_avoids_an_existing_name_in_any_case() {
    let (_directory, files) = TestFiles::new();
    files.write("UNTITLED.CSV", "a\n");
    let mut view = library(&files);

    let path = view.create_document().expect("the document is created");

    assert_eq!(path.file_name().expect("a file name"), "Untitled 2.csv");
}

#[test]
fn imports_under_the_picked_name() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join("import-notes.csv");
    std::fs::write(&source, "x,y\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    let mut view = library(&files);

    let imported = view.import_document().expect("the document is imported");

    assert_eq!(
        imported.file_name().expect("a file name"),
        "import-notes.csv"
    );
    assert!(imported.is_file());
    assert_eq!(names(&view), ["import-notes.csv"]);
}

#[test]
fn a_cancelled_import_returns_nothing() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);

    assert_eq!(view.import_document(), None);
}

#[test]
fn an_unsupported_import_is_removed_and_reported() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join("notes.md");
    std::fs::write(&source, "# notes\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    let mut view = library(&files);

    assert_eq!(view.import_document(), None);
    assert!(!files.documents_dir().join("notes.md").exists());
}

#[test]
fn a_hidden_import_is_removed_and_reported() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join(".notes.csv");
    std::fs::write(&source, "a\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    let mut view = library(&files);

    assert_eq!(view.import_document(), None);
    assert!(!files.documents_dir().join(".notes.csv").exists());
}

#[test]
fn renames_and_deletes() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");

    let renamed = view
        .rename_document(&path, "Budget")
        .expect("the document is renamed");

    assert_eq!(renamed.file_name().expect("a file name"), "Budget.csv");
    assert_eq!(names(&view), ["Budget.csv"]);

    view.delete_document(&renamed)
        .expect("the document is deleted");

    assert!(view.documents().is_empty());
}

#[test]
fn renaming_without_an_extension_keeps_the_documents_extension() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("notes.tsv", "a\tb\n");
    let mut view = library(&files);
    view.refresh();

    let renamed = view
        .rename_document(&path, "Ideas")
        .expect("the document is renamed");

    assert_eq!(renamed.file_name().expect("a file name"), "Ideas.tsv");
    assert!(renamed.is_file());
    assert!(!path.exists());
}

#[test]
fn renaming_with_an_extension_keeps_the_proposed_name() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("notes.tsv", "a\tb\n");
    let mut view = library(&files);
    view.refresh();

    let renamed = view
        .rename_document(&path, "notes.txt")
        .expect("the document is renamed");

    assert_eq!(renamed.file_name().expect("a file name"), "notes.txt");
}

#[test]
fn renaming_to_a_taken_name_gets_a_unique_name() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let first = view.create_document().expect("the first document");
    let second = view.create_document().expect("the second document");

    let renamed = view
        .rename_document(&second, "Untitled")
        .expect("the document is renamed");

    assert_eq!(renamed.file_name().expect("a file name"), "Untitled 3.csv");
    assert!(first.is_file());
    assert!(renamed.is_file());
    assert!(!second.exists());
}

#[test]
fn renaming_to_the_same_name_changes_nothing() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");

    let renamed = view
        .rename_document(&path, "Untitled.csv")
        .expect("the rename is a no-op");

    assert_eq!(renamed, path);
    assert_eq!(names(&view), ["Untitled.csv"]);
}

#[test]
fn renaming_to_an_unsupported_extension_is_refused() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");

    let error = view
        .rename_document(&path, "Notes.md")
        .expect_err("the document would leave the library");

    assert_eq!(error.message_key, "Something Went Wrong");
    assert!(path.is_file());
    assert_eq!(names(&view), ["Untitled.csv"]);
}

#[test]
fn a_rename_cannot_escape_the_documents_folder() {
    let (directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");

    for proposed in [
        ".",
        "..",
        "../escape",
        "nested/name",
        "nested\\name",
        ".hidden",
        ".csv",
        "notes.",
        "C:notes.csv",
    ] {
        let error = view
            .rename_document(&path, proposed)
            .expect_err("a path is not a document name");
        assert_eq!(error.message_key, "Something Went Wrong");
    }

    assert!(path.is_file());
    assert!(
        !directory
            .path()
            .parent()
            .expect("a parent")
            .join("escape.csv")
            .exists()
    );
    assert_eq!(names(&view), ["Untitled.csv"]);
}

#[test]
fn a_case_only_rename_keeps_the_same_file() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("Notes.csv", "a\n");
    let mut view = library(&files);
    view.refresh();

    let renamed = view
        .rename_document(&path, "notes.csv")
        .expect("the document keeps its name");

    assert_eq!(renamed.file_name().expect("a file name"), "notes.csv");
    assert_eq!(names(&view), ["notes.csv"]);
    assert!(!files.documents_dir().join("notes 2.csv").exists());
}

#[test]
fn a_blank_rename_changes_nothing() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");

    let renamed = view
        .rename_document(&path, "  \n ")
        .expect("the rename is a no-op");

    assert_eq!(renamed, path);
    assert_eq!(names(&view), ["Untitled.csv"]);
}

#[test]
fn a_failed_create_is_reported() {
    let directory = TempDir::new().expect("a temporary folder");
    let blocked = directory.path().join("blocked");
    std::fs::write(&blocked, "not a directory").expect("the blocker is written");
    let files = TestFiles::at(blocked);
    let mut view = library(&files);

    let error = view
        .create_document()
        .expect_err("a file cannot hold documents");

    assert_eq!(error.message_key, "The file could not be saved");
}

#[test]
fn a_failed_delete_is_reported_and_keeps_the_document() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");
    files.fail_delete.set(true);

    let error = view
        .delete_document(&path)
        .expect_err("the delete is reported");

    assert_eq!(error.message_key, "Something Went Wrong");
    assert!(path.exists());
    assert_eq!(names(&view), ["Untitled.csv"]);
}

#[test]
fn a_failed_rename_is_reported_and_keeps_the_document() {
    let (_directory, files) = TestFiles::new();
    let mut view = library(&files);
    let path = view.create_document().expect("the document is created");
    files.fail_rename.set(true);

    let error = view
        .rename_document(&path, "Budget")
        .expect_err("the rename is reported");

    assert_eq!(error.message_key, "Something Went Wrong");
    assert!(path.exists());
    assert_eq!(names(&view), ["Untitled.csv"]);
}

// -- Screen ---------------------------------------------------------------

#[derive(Default)]
struct Outcome(Rc<RefCell<LibraryResponse>>);

fn harness_library(
    files: &Rc<TestFiles>,
    outcome: &Outcome,
    input: &FakeTouchInput,
) -> UiHarness<'static> {
    harness_library_with_clock(files, outcome, input, LibraryClock::default())
}

fn harness_library_with_clock(
    files: &Rc<TestFiles>,
    outcome: &Outcome,
    input: &FakeTouchInput,
    clock: LibraryClock,
) -> UiHarness<'static> {
    let mut view = library_with_clock(files, clock);
    let mut input = input.clone();
    let outcome = outcome.0.clone();
    UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = view.show(ui, &mut input);
        let mut last = outcome.borrow_mut();
        if response.open.is_some() {
            last.open = response.open;
        }
        last.settings |= response.settings;
        last.about |= response.about;
    })
}

#[test]
fn the_empty_library_explains_how_to_start() {
    let (_directory, files) = TestFiles::new();
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    assert!(harness.query_by_label(tr(EN, "No Documents")).is_some());
    assert!(
        harness
            .query_by_label(tr(EN, "Create a document or import a CSV file."))
            .is_some()
    );
}

#[test]
fn the_plus_menu_creates_a_document_and_reports_it() {
    let (_directory, files) = TestFiles::new();
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Add Document")).click();
    harness.run();
    harness.get_by_label(tr(EN, "New Document")).click();
    harness.run();

    let opened = outcome
        .0
        .borrow()
        .open
        .clone()
        .expect("the new document opens");
    assert_eq!(opened.file_name().expect("a file name"), "Untitled.csv");
    assert!(opened.is_file());
    assert!(harness.query_by_label(tr(EN, "No Documents")).is_none());
}

#[test]
fn the_plus_menu_offers_new_import_and_settings() {
    let (_directory, files) = TestFiles::new();
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Add Document")).click();
    harness.run();

    assert!(harness.query_by_label(tr(EN, "New Document")).is_some());
    assert!(harness.query_by_label(tr(EN, "Import…")).is_some());
    assert!(
        harness
            .query_all_by_label(tr(EN, "Settings"))
            .next()
            .is_some()
    );
}

#[test]
fn the_plus_menu_imports_the_picked_document() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join("from-picker.csv");
    std::fs::write(&source, "x,y\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Add Document")).click();
    harness.run();
    harness.get_by_label(tr(EN, "Import…")).click();
    harness.run();

    let opened = outcome.0.borrow().open.clone().expect("the document opens");
    assert_eq!(opened.file_name().expect("a file name"), "from-picker.csv");
}

#[test]
fn an_async_picker_reports_the_import_when_it_finishes() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join("later.csv");
    std::fs::write(&source, "x,y\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    files.in_progress.set(true);
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Add Document")).click();
    harness.run();
    harness.get_by_label(tr(EN, "Import…")).click();
    harness.run();
    assert!(
        outcome.0.borrow().open.is_none(),
        "an open picker does not open a document yet"
    );

    files.in_progress.set(false);
    harness.run();

    let opened = outcome
        .0
        .borrow()
        .open
        .clone()
        .expect("the finished import opens");
    assert_eq!(opened.file_name().expect("a file name"), "later.csv");
}

#[test]
fn an_unsupported_import_shows_a_localized_banner() {
    let (_directory, files) = TestFiles::new();
    let source_directory = TempDir::new().expect("a temporary import folder");
    let source = source_directory.path().join("notes.md");
    std::fs::write(&source, "# notes\n").expect("the import source is written");
    *files.picker.borrow_mut() = Some(source);
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Add Document")).click();
    harness.run();
    harness.get_by_label(tr(EN, "Import…")).click();
    harness.run();

    assert!(
        harness
            .query_by_label(tr(EN, "The file could not be opened"))
            .is_some()
    );
    assert!(!files.documents_dir().join("notes.md").exists());
}

#[test]
fn the_gear_opens_settings() {
    let (_directory, files) = TestFiles::new();
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "Settings")).click();
    harness.run();

    assert!(outcome.0.borrow().settings);
}

#[test]
fn the_overflow_opens_about() {
    let (_directory, files) = TestFiles::new();
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label(tr(EN, "More")).click();
    harness.run();
    harness.get_by_label(tr(EN, "About")).click();
    harness.run();

    assert!(outcome.0.borrow().about);
}

#[test]
fn a_row_tap_returns_the_document_path() {
    let (_directory, files) = TestFiles::new();
    files.write("Budget.csv", "a,b\n");
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    harness.get_by_label("Budget").click();
    harness.run();

    assert_eq!(
        outcome.0.borrow().open.as_deref(),
        Some(files.documents_dir().join("Budget.csv").as_path())
    );
}

#[test]
fn swiping_a_row_asks_before_deleting() {
    let (_directory, files) = TestFiles::new();
    files.write("Notes.csv", "a\n");
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    let rect = harness.get_by_label("Notes").rect();
    let start = rect.center();
    let end = egui::pos2(rect.center().x - 100.0, rect.center().y);
    harness.drag_at(start);
    harness.hover_at(end);
    harness.run();
    harness.drop_at(end);
    harness.run();

    harness.get_by_label(tr(EN, "Delete")).click();
    harness.run();

    assert!(harness.query_by_label(tr(EN, "Delete Document?")).is_some());
    assert!(files.documents_dir().join("Notes.csv").is_file());

    harness.get_by_label(tr(EN, "Delete")).click();
    harness.run();

    assert!(!files.documents_dir().join("Notes.csv").exists());
    assert!(harness.query_by_label(tr(EN, "No Documents")).is_some());
}

#[test]
fn cancelling_the_delete_confirmation_keeps_the_document() {
    let (_directory, files) = TestFiles::new();
    files.write("Notes.csv", "a\n");
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    let rect = harness.get_by_label("Notes").rect();
    let start = rect.center();
    let end = egui::pos2(rect.center().x - 100.0, rect.center().y);
    harness.drag_at(start);
    harness.hover_at(end);
    harness.run();
    harness.drop_at(end);
    harness.run();
    harness.get_by_label(tr(EN, "Delete")).click();
    harness.run();

    harness.get_by_label(tr(EN, "Cancel")).click();
    harness.run();

    assert!(files.documents_dir().join("Notes.csv").is_file());
    assert!(harness.query_by_label("Notes").is_some());
}

#[test]
fn the_context_menu_renames_through_the_native_editor() {
    let (_directory, files) = TestFiles::new();
    files.write("Notes.csv", "a\n");
    let outcome = Outcome::default();
    let input = FakeTouchInput::default();
    let mut harness = harness_library(&files, &outcome, &input);
    harness.run();

    harness.get_by_label("Notes").click_secondary();
    harness.run();
    harness.get_by_label(tr(EN, "Rename")).click();
    harness.run();

    let request = input.request().expect("the editor opens");
    assert_eq!(request.text, "Notes.csv");
    assert!(!request.multiline);
    input.finish(TouchTextInputOutcome::Committed("Budget".to_owned()));
    harness.run();

    assert!(files.documents_dir().join("Budget.csv").is_file());
    assert!(!files.documents_dir().join("Notes.csv").exists());
}

#[test]
fn cancelling_the_native_editor_keeps_the_name() {
    let (_directory, files) = TestFiles::new();
    files.write("Notes.csv", "a\n");
    let outcome = Outcome::default();
    let input = FakeTouchInput::default();
    let mut harness = harness_library(&files, &outcome, &input);
    harness.run();

    harness.get_by_label("Notes").click_secondary();
    harness.run();
    harness.get_by_label(tr(EN, "Rename")).click();
    harness.run();
    input.finish(TouchTextInputOutcome::Cancelled);
    harness.run();

    assert!(files.documents_dir().join("Notes.csv").is_file());
}

#[test]
fn a_platform_error_shows_a_localized_banner_and_dismisses() {
    let (_directory, files) = TestFiles::new();
    files.pending_error.replace(Some(LocalizedError::new(
        "The file could not be saved",
        "the disk is full",
    )));
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    assert!(
        harness
            .query_by_label(tr(EN, "The file could not be saved"))
            .is_some()
    );

    harness.get_by_label(tr(EN, "OK")).click();
    harness.run();

    assert!(
        harness
            .query_by_label(tr(EN, "The file could not be saved"))
            .is_none()
    );
}

#[test]
fn the_library_subtitle_uses_the_locale_for_size_and_date() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("notes.csv", "a,b\n1,2\n");
    set_modified(&path, 1_700_000_000);
    let outcome = Outcome::default();
    let mut harness = harness_library(&files, &outcome, &FakeTouchInput::default());
    harness.run();

    // 8 bytes and a 14 November 2023 timestamp: the English short form is
    // month/day/year with a 24-hour clock.
    assert!(
        harness.query_by_label("8 B · 11/14/2023, 22:13").is_some(),
        "the row shows the localized size and short date"
    );
}

#[test]
fn the_row_uses_the_platform_clock() {
    let (_directory, files) = TestFiles::new();
    let path = files.write("notes.csv", "a,b\n1,2\n");
    set_modified(&path, 1_700_000_000);
    let outcome = Outcome::default();
    let mut harness = harness_library_with_clock(
        &files,
        &outcome,
        &FakeTouchInput::default(),
        LibraryClock::new(-480, true),
    );
    harness.run();

    assert!(
        harness
            .query_by_label("8 B · 11/14/2023, 2:13 PM")
            .is_some(),
        "the row uses the platform's local offset and 12-hour clock"
    );
}

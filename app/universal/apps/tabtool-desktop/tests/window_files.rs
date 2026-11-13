//! Window and app wiring for drops, export panels and Open Recent.

mod support;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Context, ViewportId};
use tabtool_core::CsvConfiguration;
use tabtool_desktop::app::DesktopApp;
use tabtool_desktop::documents::{CloseChoice, DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::harness::kittest::{NodeT, Queryable};
use tabtool_ui::{Theme, UiHarness};

use support::{FakeDialogs, MemoryStorage};

fn write_file(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, bytes).expect("the fixture file is written");
    path
}

fn window(
    document: tabtool_document::Document,
    settings: Settings,
    storage: SharedStorage,
    dialogs: SharedDialogs,
) -> DocumentWindow {
    DocumentWindow::new(
        ViewportId::from_hash_of("window-files"),
        document,
        "Untitled".to_owned(),
        Language::En,
        true,
        WindowServices {
            platform: DesktopPlatform,
            dialogs,
            storage,
            settings,
        },
    )
}

#[test]
fn dropping_a_file_on_a_dirty_window_asks_and_dont_save_opens_it() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"a,b\n3,4\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    document.set_cell(0, 0, "edited");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new.clone()]);
    assert!(window.has_open_prompt(), "a dirty window must ask first");

    window.answer_open_prompt(CloseChoice::DontSave);
    window.tick(Instant::now(), &Context::default());

    assert_eq!(window.title(), "new.csv");
    assert!(window.take_opened_paths().contains(&new));
}

#[test]
fn dropping_a_file_on_a_clean_window_opens_it_immediately() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"a,b\n3,4\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new.clone()]);

    assert!(!window.has_open_prompt());
    assert_eq!(window.title(), "new.csv");
    assert!(window.take_opened_paths().contains(&new));
}

#[test]
fn cancelling_the_open_prompt_keeps_the_document() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"a,b\n3,4\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    document.set_cell(0, 0, "edited");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new]);
    window.answer_open_prompt(CloseChoice::Cancel);
    window.tick(Instant::now(), &Context::default());

    assert_eq!(window.title(), "old.csv");
    assert!(window.is_dirty());
}

#[test]
fn dropping_a_missing_file_shows_the_read_error_overlay() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let missing = directory.path().join("missing.csv");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![missing.clone()]);

    assert_eq!(window.title(), "missing.csv");
    assert_eq!(
        window.with_document(|document| *document.phase()),
        tabtool_document::DocumentPhase::UnreadableFile
    );
    assert!(window.take_opened_paths().is_empty());
}

#[test]
fn dropping_a_non_table_file_is_ignored() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let image = write_file(directory.path(), "image.png", b"not a table");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![image]);

    assert!(!window.has_open_prompt());
    assert_eq!(window.title(), "old.csv");
}

#[test]
fn the_toolbar_export_button_opens_the_panel_and_writes_the_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = write_file(directory.path(), "Sales.csv", b"a,b\n1,2\n");
    let export = directory.path().join("Sales-export.csv");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs = FakeDialogs::scripted([Some(export.clone())]);
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&source, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = Rc::new(window(
        document,
        Settings::default(),
        storage,
        shared_dialogs,
    ));
    let shown = Rc::clone(&window);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        shown.render(ui);
    });

    harness.run();
    harness.get_by_label("Export").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Write Header Row").is_some(),
        "the export panel must open"
    );
    harness
        .query_all_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .last()
        .expect("the panel has an Export button")
        .click();
    harness.run();

    assert!(
        std::fs::read_to_string(&export).is_ok(),
        "the export must write the selected path"
    );
    assert_eq!(
        dialogs.saves(),
        [("Sales-export.csv".to_owned(), "csv".to_owned())]
    );
}

#[test]
fn answering_save_on_the_open_prompt_saves_then_opens() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let new = write_file(directory.path(), "new.csv", b"a,b\n3,4\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    document.set_cell(0, 0, "edited");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new.clone()]);
    window.answer_open_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &Context::default());

    assert_eq!(window.title(), "new.csv");
    assert_eq!(
        std::fs::read_to_string(&old).expect("the old file is readable"),
        "\"edited\",\"b\"\n\"1\",\"2\""
    );
    assert!(window.take_opened_paths().contains(&new));
}

#[test]
fn answering_save_for_an_untitled_document_uses_the_panel_then_opens() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let saved = directory.path().join("Untitled.csv");
    let new = write_file(directory.path(), "new.csv", b"a,b\n3,4\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs = FakeDialogs::scripted([Some(saved.clone())]);
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .new_document(CsvConfiguration::default())
        .expect("create the document");
    document.set_cell(0, 0, "edited");
    let window = window(document, Settings::default(), storage, shared_dialogs);

    window.drop_files(vec![new.clone()]);
    window.answer_open_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &Context::default());

    assert_eq!(
        dialogs.saves(),
        [("Untitled.csv".to_owned(), "csv".to_owned())]
    );
    assert_eq!(window.title(), "new.csv");
    assert!(saved.exists(), "the untitled document is saved first");
}

#[test]
fn a_drop_during_a_large_save_waits_and_opens_after_it_settles() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let big = write_file(directory.path(), "big.csv", b"a,b\n1,2\n3,4\n");
    let new = write_file(directory.path(), "after-save.csv", b"a,b\n5,6\n");

    let mut document = support::large_ready(&big);
    document
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    document
        .start_large_save(None)
        .expect("start the large save");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new.clone()]);

    assert!(window.is_busy(), "the drop must wait for the running save");
    assert_eq!(window.title(), "big.csv");

    let ctx = Context::default();
    for _ in 0..400 {
        window.tick(Instant::now(), &ctx);
        if window.title() == "after-save.csv" {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(window.title(), "after-save.csv");
    assert!(window.take_opened_paths().contains(&new));
}

#[test]
fn saving_then_opening_waits_for_the_large_save_to_finish() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let big = write_file(directory.path(), "big-save.csv", b"a,b\n1,2\n3,4\n");
    let new = write_file(directory.path(), "after-save.csv", b"a,b\n5,6\n");

    let mut document = support::large_ready(&big);
    document
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![new.clone()]);
    assert!(window.has_open_prompt(), "a dirty large document must ask");
    window.answer_open_prompt(CloseChoice::Save);

    let ctx = Context::default();
    window.tick(Instant::now(), &ctx);
    assert!(
        window.is_busy() || window.title() == "after-save.csv",
        "save-then-open must show the save or already have opened"
    );
    for _ in 0..400 {
        window.tick(Instant::now(), &ctx);
        if window.title() == "after-save.csv" {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(window.title(), "after-save.csv");
    assert!(
        std::fs::read_to_string(&big)
            .expect("the large save is readable")
            .contains("edited"),
        "save-then-open must finish the large save first"
    );
    assert!(window.take_opened_paths().contains(&new));
}

#[test]
fn extra_dropped_files_are_returned_for_new_windows() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let old = write_file(directory.path(), "old.csv", b"a,b\n1,2\n");
    let second = write_file(directory.path(), "second.csv", b"a,b\n3,4\n");
    let third = write_file(directory.path(), "third.csv", b"a,b\n5,6\n");

    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&old, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = window(document, Settings::default(), storage, dialogs);

    window.drop_files(vec![second.clone(), third.clone()]);

    assert_eq!(window.title(), "second.csv");
    assert_eq!(window.take_dropped_files(), [third]);
}

#[test]
fn the_app_opens_extra_dropped_files_as_new_windows() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let one = write_file(directory.path(), "one.csv", b"a,b\n1,2\n");
    let two = write_file(directory.path(), "two.csv", b"a,b\n3,4\n");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut app = DesktopApp::with_dialogs(Settings::default(), DesktopPlatform, storage, dialogs);

    app.open_dropped(&[one.clone(), two.clone()]);

    assert_eq!(app.documents().len(), 2);
    assert_eq!(app.documents()[0].title(), "one.csv");
    assert_eq!(app.documents()[1].title(), "two.csv");
    assert_eq!(app.settings.recent_documents, [two, one]);
}

#[test]
fn the_export_menu_action_opens_the_panel() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = write_file(directory.path(), "menu-export.csv", b"a,b\n1,2\n");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut app = DesktopApp::with_dialogs(Settings::default(), DesktopPlatform, storage, dialogs);
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(
            &source,
            &tabtool_document::StdStorage,
            Some(CsvConfiguration::default()),
            false,
        )
        .expect("load the fixture");
    app.add_document(document);

    let handle = Rc::new(RefCell::new(app));
    let ui_app = Rc::clone(&handle);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = ui_app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();

    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Export…").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_by_label("Write Header Row").is_some(),
        "the export panel must open from the menu"
    );
}

#[test]
fn an_ambiguous_export_confirms_before_the_save_panel_and_writes_after() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = write_file(directory.path(), "ambiguous.csv", b"1.5\n");
    let export = directory.path().join("ambiguous-export.csv");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs = FakeDialogs::scripted([Some(export.clone())]);
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let no_quote = CsvConfiguration {
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = tabtool_document::Document::new(no_quote.clone());
    document
        .load_from_path(&source, &*storage, Some(no_quote), false)
        .expect("load the fixture");
    let window = Rc::new(window(
        document,
        Settings::default(),
        storage,
        shared_dialogs,
    ));
    let shown = Rc::clone(&window);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        shown.render(ui);
    });
    harness.run();
    harness.get_by_label("Export").click();
    harness.run();
    harness.run();

    harness
        .query_all_by_role_and_label(egui::accesskit::Role::ComboBox, "Decimal")
        .last()
        .expect("the panel has a Decimal picker")
        .click();
    harness.run();
    harness.get_by_label("Comma (,)").click();
    harness.run();
    harness
        .query_all_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .last()
        .expect("the panel has an Export button")
        .click();
    harness.run();

    assert!(
        harness.query_by_label("Export Anyway?").is_some(),
        "an ambiguous export must ask first"
    );
    assert!(
        dialogs.saves().is_empty(),
        "the panel is not shown before asking"
    );
    assert!(!export.exists());

    harness.get_by_label("Export Anyway").click();
    harness.run();

    assert_eq!(
        std::fs::read_to_string(&export).expect("the export is readable"),
        "1,5"
    );
    assert_eq!(
        window.with_document(|document| document.table().expect("table").cell(0, 0)),
        "1.5",
        "export must not change the document"
    );
}

#[test]
fn cancelling_the_export_save_panel_returns_to_the_document() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = write_file(directory.path(), "cancel-export.csv", b"a,b\n1,2\n");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&source, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = Rc::new(window(document, Settings::default(), storage, dialogs));
    let shown = Rc::clone(&window);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        shown.render(ui);
    });
    harness.run();
    harness.get_by_label("Export").click();
    harness.run();
    harness.run();
    harness
        .query_all_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .last()
        .expect("the panel has an Export button")
        .click();
    harness.run();

    assert!(
        harness.query_by_label("Write Header Row").is_none(),
        "cancelling the save panel must close the export sheet"
    );
    assert!(harness.query_by_label("Export Not Completed").is_none());
}

#[test]
fn an_unencodable_export_shows_the_typed_alert_without_a_save_panel() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source = write_file(directory.path(), "cyrillic.csv", "ж\n".as_bytes());
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs = FakeDialogs::cancelled();
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .load_from_path(&source, &*storage, Some(CsvConfiguration::default()), false)
        .expect("load the fixture");
    let window = Rc::new(window(
        document,
        Settings::default(),
        storage,
        shared_dialogs,
    ));
    let shown = Rc::clone(&window);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        shown.render(ui);
    });
    harness.run();
    harness.get_by_label("Export").click();
    harness.run();
    harness.run();

    harness
        .query_all_by_role_and_label(egui::accesskit::Role::ComboBox, "Encoding")
        .last()
        .expect("the panel has an Encoding picker")
        .click();
    harness.run();
    harness.get_by_label("Western (Windows Latin 1)").click();
    harness.run();
    harness
        .query_all_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .last()
        .expect("the panel has an Export button")
        .click();
    harness.run();

    assert!(
        harness.query_by_label("Export Not Completed").is_some(),
        "an unencodable export must explain the failure"
    );
    assert!(dialogs.saves().is_empty());
    assert_eq!(
        window.with_document(|document| document.table().expect("table").cell(0, 0)),
        "ж"
    );
}

#[test]
fn the_open_menu_adds_a_document_and_updates_recent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "menu-open.csv", b"a,b\n1,2\n");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs = FakeDialogs::scripted_opens([vec![path.clone()]]);
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let app = Rc::new(RefCell::new(DesktopApp::with_dialogs(
        Settings::default(),
        DesktopPlatform,
        storage,
        shared_dialogs,
    )));

    let ui_app = Rc::clone(&app);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = ui_app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();

    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Open…").click();
    harness.run();

    let app = app.borrow();
    assert_eq!(app.documents().len(), 1);
    assert_eq!(app.documents()[0].title(), "menu-open.csv");
    assert_eq!(app.settings.recent_documents, [path]);
}

#[test]
fn the_open_recent_item_lists_the_saved_paths() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "recent-menu.csv", b"a,b\n1,2\n");
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let shared: SharedStorage = Arc::new(storage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let settings = Settings {
        recent_documents: vec![path.clone()],
        ..Settings::default()
    };
    let mut app = DesktopApp::with_dialogs(settings, DesktopPlatform, shared, dialogs);
    app.new_document();
    let handle = Rc::new(RefCell::new(app));
    let ui_app = Rc::clone(&handle);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = ui_app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();

    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Open Recent").click();
    harness.run();

    assert!(
        harness.query_by_label("recent-menu.csv").is_some(),
        "the recent chooser lists the file name"
    );
    assert!(handle.borrow().is_recent_picker_visible());
}

#[test]
fn a_tick_prunes_missing_recent_paths_and_disables_open_recent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = write_file(directory.path(), "vanished.csv", b"a,b\n1,2\n");
    let storage: SharedStorage = Arc::new(tabtool_document::StdStorage);
    let dialogs: SharedDialogs = FakeDialogs::cancelled();
    let settings = Settings {
        recent_documents: vec![path.clone()],
        ..Settings::default()
    };
    let app = DesktopApp::with_dialogs(settings, DesktopPlatform, storage, dialogs);
    std::fs::remove_file(&path).expect("the fixture is removed");
    let handle = Rc::new(RefCell::new(app));
    let ui_app = Rc::clone(&handle);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = ui_app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();

    {
        let app = handle.borrow();
        assert!(app.files.recent().is_empty());
        assert!(app.settings.recent_documents.is_empty());
        assert!(!app.files.has_recent());
    }

    harness.get_by_label("File").click();
    harness.run();
    assert!(
        harness
            .get_by_label("Open Recent")
            .accesskit_node()
            .is_disabled(),
        "Open Recent must disable once every recent path is gone"
    );
}

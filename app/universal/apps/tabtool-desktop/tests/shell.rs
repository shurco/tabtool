//! Window model, close/quit decisions and the shell's rendered placeholders.

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tabtool_core::CsvConfiguration;
use tabtool_desktop::app::{DesktopApp, QuitDecision, quit_decision};
use tabtool_desktop::documents::{
    CloseDecision, CloseSave, DOCUMENT_MIN_INNER_SIZE, close_decision,
};
use tabtool_desktop::platform::{SharedStorage, interface_language, resolve_language};
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::{Theme, UiHarness};

use support::MemoryStorage;

fn app() -> DesktopApp {
    let storage: SharedStorage = Arc::new(MemoryStorage::new(b""));
    DesktopApp::with_storage(Settings::default(), DesktopPlatform, storage)
}

fn app_handle() -> Rc<RefCell<DesktopApp>> {
    Rc::new(RefCell::new(app()))
}

fn render(handle: &Rc<RefCell<DesktopApp>>) -> UiHarness<'static> {
    let app = Rc::clone(handle);
    UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    })
}

#[test]
fn new_documents_get_numbered_untitled_titles() {
    let mut app = app();
    app.new_document();
    app.new_document();
    app.new_document();

    let titles: Vec<String> = app.documents().iter().map(|w| w.title()).collect();
    assert_eq!(titles, ["Untitled", "Untitled 2", "Untitled 3"]);
}

#[test]
fn a_file_backed_document_uses_its_file_name() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let document = support::loaded(&storage, CsvConfiguration::default());
    let mut app = app();
    app.add_document(document);

    assert_eq!(app.documents()[0].title(), "document.csv");
    assert!(!app.documents()[0].is_dirty());
}

#[test]
fn each_document_has_a_unique_viewport_id() {
    let mut app = app();
    app.new_document();
    app.new_document();

    let documents = app.documents();
    assert_ne!(documents[0].viewport_id(), documents[1].viewport_id());
}

#[test]
fn document_windows_keep_the_apple_minimum_size() {
    assert_eq!(DOCUMENT_MIN_INNER_SIZE, [640.0, 400.0]);
}

#[test]
fn a_clean_document_closes_without_a_prompt() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let document = support::loaded(&storage, CsvConfiguration::default());

    assert_eq!(close_decision(&document), CloseDecision::Close);
}

#[test]
fn a_dirty_document_asks_before_closing() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");

    assert_eq!(close_decision(&document), CloseDecision::Ask);
}

#[test]
fn an_untitled_document_asks_before_closing() {
    let document = support::untitled(CsvConfiguration::default());

    assert_eq!(close_decision(&document), CloseDecision::Ask);
}

#[test]
fn a_running_large_save_reports_busy() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("big.csv");
    std::fs::write(&path, b"a,b\n1,2\n").expect("fixture file is written");

    let storage = MemoryStorage::new(b"");
    let app_storage: SharedStorage = Arc::new(storage);
    let mut app = DesktopApp::with_storage(Settings::default(), DesktopPlatform, app_storage);

    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
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
    assert!(document.is_dirty());

    app.add_document(document);
    assert_eq!(app.documents()[0].close_decision(), CloseDecision::Ask);
    assert_eq!(
        app.documents()[0].begin_close_save(),
        CloseSave::Saving,
        "a large close uses the streaming save job"
    );
    assert_eq!(app.documents()[0].close_decision(), CloseDecision::Busy);
    assert_eq!(quit_decision(app.documents()), QuitDecision::Busy);

    app.documents()[0].cancel_close_save();
}

#[test]
fn quit_is_immediate_when_every_document_is_clean() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut app = app();
    app.add_document(support::loaded(&storage, CsvConfiguration::default()));
    app.add_document(support::loaded(&storage, CsvConfiguration::default()));

    assert_eq!(quit_decision(app.documents()), QuitDecision::Quit);
}

#[test]
fn quit_asks_when_any_document_is_dirty() {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut app = app();
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    app.add_document(document);

    assert_eq!(quit_decision(app.documents()), QuitDecision::Ask);
}

#[test]
fn the_shell_renders_the_empty_state_and_new_document_command() {
    let handle = app_handle();
    let mut harness = render(&handle);

    harness.run();
    assert!(harness.query_by_label("No Documents").is_some());
    assert!(
        harness
            .query_by_label("Create a document or import a CSV file.")
            .is_some()
    );

    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("New").click();
    harness.run();

    assert_eq!(handle.borrow().documents().len(), 1);
    assert!(harness.query_by_label("File").is_some());
}

#[test]
fn the_shell_renders_the_document_placeholder() {
    let handle = app_handle();
    {
        let storage = MemoryStorage::new(b"a,b\n1,2\n");
        handle
            .borrow_mut()
            .add_document(support::loaded(&storage, CsvConfiguration::default()));
    }
    let mut harness = render(&handle);

    harness.run();

    assert!(harness.query_by_label("File").is_some());
    assert!(harness.query_all_by_label("document.csv").next().is_some());
    assert!(harness.query_by_label("CSV Table").is_some());
}

#[test]
fn the_settings_and_about_windows_are_placeholders() {
    let handle = app_handle();
    handle.borrow_mut().open_settings();
    handle.borrow_mut().open_about();
    let mut harness = render(&handle);

    harness.run();

    assert!(harness.query_all_by_label("Settings").next().is_some());
    assert!(harness.query_all_by_label("About TabTool").next().is_some());
}

#[test]
fn quitting_with_a_dirty_document_shows_the_save_alert() {
    let handle = app_handle();
    {
        let storage = MemoryStorage::new(b"a,b\n1,2\n");
        let mut document = support::loaded(&storage, CsvConfiguration::default());
        document.set_cell(0, 0, "edited");
        handle.borrow_mut().add_document(document);
    }
    let mut harness = render(&handle);
    harness.run();

    harness.get_by_label("TabTool").click();
    harness.run();
    harness.get_by_label("Quit").click();
    harness.run();
    harness.run();

    assert!(
        harness.query_all_by_label("Don’t Save").next().is_some(),
        "an unsaved document must ask before quitting"
    );
    assert!(harness.query_all_by_label("Cancel").next().is_some());
}

#[test]
fn the_interface_language_follows_the_setting_then_the_system() {
    let settings = Settings {
        app_language: tabtool_i18n::Language::Fr,
        ..Settings::default()
    };
    assert_eq!(
        interface_language(&settings, &DesktopPlatform),
        tabtool_i18n::Language::Fr
    );

    let settings = Settings {
        app_language: tabtool_i18n::Language::System,
        ..Settings::default()
    };
    assert_eq!(
        resolve_language(settings.app_language, &["de-DE".to_owned()]),
        tabtool_i18n::Language::De
    );
    assert_eq!(
        resolve_language(settings.app_language, &["xx-YY".to_owned()]),
        tabtool_i18n::Language::En
    );
}

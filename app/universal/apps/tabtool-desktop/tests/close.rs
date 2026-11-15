//! Save / Don't Save / Cancel with injected dialogs and storage.

mod support;

use std::sync::Arc;
use std::time::Instant;

use egui::{Context, ViewportId};
use tabtool_core::CsvConfiguration;
use tabtool_desktop::documents::{CloseChoice, DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;

use support::{FakeDialogs, MemoryStorage};

fn window(
    document: tabtool_document::Document,
    untitled_name: &str,
    storage: &Arc<MemoryStorage>,
    dialogs: &Arc<FakeDialogs>,
) -> DocumentWindow {
    let storage_clone = Arc::clone(storage);
    let shared_storage: SharedStorage = storage_clone;
    let dialogs_clone = Arc::clone(dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    DocumentWindow::new(
        ViewportId::from_hash_of("test-document"),
        document,
        untitled_name.to_owned(),
        Language::En,
        true,
        WindowServices {
            platform: DesktopPlatform,
            dialogs: shared_dialogs,
            storage: shared_storage,
            settings: tabtool_settings::Settings::default(),
            prefers_chinese: false,
        },
    )
}

fn dirty_document(storage: &MemoryStorage) -> tabtool_document::Document {
    let mut document = support::loaded(storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    document
}

#[test]
fn save_writes_and_closes_the_window() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(dirty_document(&storage), "Untitled", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    assert!(window.has_close_prompt());
    window.answer_close_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &ctx);

    assert!(window.should_close());
    assert!(!window.is_dirty());
    assert_eq!(storage.writes(), 1);
    assert!(
        dialogs.saves().is_empty(),
        "a file-backed Save needs no panel"
    );
}

#[test]
fn dont_save_closes_without_writing() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(dirty_document(&storage), "Untitled", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    window.answer_close_prompt(CloseChoice::DontSave);
    window.tick(Instant::now(), &ctx);

    assert!(window.should_close());
    assert_eq!(storage.writes(), 0);
    assert!(window.is_dirty(), "the discarded edit stays in memory");
}

#[test]
fn cancel_keeps_the_window_open_and_dirty() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(dirty_document(&storage), "Untitled", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    window.answer_close_prompt(CloseChoice::Cancel);
    window.tick(Instant::now(), &ctx);

    assert!(!window.should_close());
    assert!(window.is_dirty());
    assert_eq!(storage.writes(), 0);
}

#[test]
fn an_untitled_save_uses_the_injected_panel_and_binds_the_path() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("report.csv");
    let storage = Arc::new(MemoryStorage::new(b""));
    let dialogs = FakeDialogs::scripted([Some(path.clone())]);
    let mut document = support::untitled(CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    let window = window(document, "Untitled 2", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    window.answer_close_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &ctx);

    assert_eq!(
        dialogs.saves(),
        [("Untitled 2.csv".to_owned(), "csv".to_owned())]
    );
    assert_eq!(storage.writes(), 1);
    assert!(window.should_close());
    assert!(!window.is_dirty());
    assert_eq!(window.title(), "report.csv");
}

#[test]
fn cancelling_the_untitled_save_panel_cancels_the_close() {
    let storage = Arc::new(MemoryStorage::new(b""));
    let dialogs = FakeDialogs::cancelled();
    let document = support::untitled(CsvConfiguration::default());
    let window = window(document, "Untitled", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    window.answer_close_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &ctx);

    assert!(!window.should_close());
    assert!(window.is_dirty());
    assert_eq!(storage.writes(), 0);
    assert_eq!(dialogs.saves().len(), 1);
}

#[test]
fn a_failed_save_keeps_the_window_open() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    storage.fail_writes(true);
    let dialogs = FakeDialogs::cancelled();
    let window = window(dirty_document(&storage), "Untitled", &storage, &dialogs);
    let ctx = Context::default();

    window.request_close();
    window.answer_close_prompt(CloseChoice::Save);
    window.tick(Instant::now(), &ctx);

    assert!(!window.should_close());
    assert!(window.is_dirty());
    assert_eq!(storage.writes(), 1);
}

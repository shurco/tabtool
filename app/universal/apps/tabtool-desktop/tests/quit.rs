//! Quit attempts that are cancelled must not leave windows closing.

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use egui::{ViewportEvent, ViewportId};
use tabtool_core::CsvConfiguration;
use tabtool_desktop::app::{DesktopApp, QuitResponse};
use tabtool_desktop::documents::{DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_document::AUTOSAVE_DELAY;
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_settings::Settings;
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::{Theme, UiHarness};

use support::{FakeDialogs, MemoryStorage};

fn app(storage: SharedStorage, dialogs: SharedDialogs) -> Rc<RefCell<DesktopApp>> {
    Rc::new(RefCell::new(DesktopApp::with_dialogs(
        Settings::default(),
        DesktopPlatform,
        storage,
        dialogs,
    )))
}

fn window(
    document: tabtool_document::Document,
    storage: SharedStorage,
    dialogs: SharedDialogs,
) -> DocumentWindow {
    DocumentWindow::new(
        ViewportId::from_hash_of("quit-editor"),
        document,
        "document.csv".to_owned(),
        Language::En,
        true,
        WindowServices {
            platform: DesktopPlatform,
            dialogs,
            storage,
            settings: Settings::default(),
            prefers_chinese: false,
        },
    )
}

#[test]
fn autosave_waits_while_the_quit_prompt_is_visible() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let storage_shared: SharedStorage = storage.clone();
    let dialogs_shared: SharedDialogs = dialogs.clone();
    let app = app(storage_shared, dialogs_shared);
    let mut document = support::loaded(&storage, CsvConfiguration::default());
    document.set_cell(0, 0, "edited");
    app.borrow_mut().add_document(document);

    let clock = Rc::new(Cell::new(Instant::now()));
    let closure_clock = Rc::clone(&clock);
    let handle = Rc::clone(&app);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = handle.borrow_mut();
        app.tick_at(closure_clock.get(), ui.ctx());
        app.show(ui);
    });
    harness.run();
    assert_eq!(storage.writes(), 0, "the first pass only arms autosave");

    app.borrow_mut().request_quit(&harness.ctx);
    assert!(app.borrow().is_quit_prompt_visible());

    clock.set(clock.get() + AUTOSAVE_DELAY);
    harness.run();
    assert_eq!(
        storage.writes(),
        0,
        "the debounce must not fire under the quit prompt"
    );

    app.borrow_mut()
        .answer_quit_prompt(QuitResponse::DontSave, &harness.ctx);
    clock.set(clock.get() + AUTOSAVE_DELAY);
    harness.run();

    assert_eq!(storage.writes(), 0, "Don’t Save must discard the edit");
}

#[test]
fn an_os_quit_flushes_the_text_of_every_open_editor() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\nx,y\n"));
    let dialogs = FakeDialogs::cancelled();
    let storage_shared: SharedStorage = storage.clone();
    let dialogs_shared: SharedDialogs = dialogs.clone();
    let window_storage: SharedStorage = storage.clone();
    let window_dialogs: SharedDialogs = dialogs.clone();
    let app = app(storage_shared, dialogs_shared);
    let document = support::loaded(&storage, CsvConfiguration::default());
    let window = window(document, window_storage, window_dialogs);
    let handle = window.clone();
    let mut window_harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        handle.tick(Instant::now(), ui.ctx());
        handle.render(ui);
    });
    window_harness.run();
    window_harness.get_by_label("x").click();
    window_harness.run();
    window_harness.get_by_label("x").click();
    window_harness.run();
    window_harness.run();
    window_harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text("edited");
    window_harness.run();
    drop(window_harness);

    app.borrow_mut().documents.push(window);

    let app_handle = Rc::clone(&app);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = app_handle.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.step();
    harness
        .input_mut()
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .events
        .push(ViewportEvent::Close);
    harness.step();

    assert!(
        app.borrow().is_quit_prompt_visible(),
        "the editor text must make the document dirty before the quit decision"
    );

    app.borrow_mut()
        .answer_quit_prompt(QuitResponse::Save, &harness.ctx);
    harness.run_steps(4);

    assert_eq!(storage.writes(), 1);
    assert!(
        String::from_utf8(storage.bytes())
            .expect("the saved CSV is UTF-8")
            .contains("xedited"),
        "the typed text must be on disk"
    );
}

#[test]
fn cancelling_save_all_stops_large_windows_from_closing() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("big.csv");
    std::fs::write(&path, b"a,b\n1,2\n").expect("fixture file is written");

    let storage = Arc::new(MemoryStorage::new(b""));
    let dialogs = FakeDialogs::cancelled();
    let storage_clone = Arc::clone(&storage);
    let shared_storage: SharedStorage = storage_clone;
    let dialogs_clone = Arc::clone(&dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    let app = Rc::new(RefCell::new(DesktopApp::with_dialogs(
        Settings::default(),
        DesktopPlatform,
        shared_storage,
        shared_dialogs,
    )));

    {
        let mut app = app.borrow_mut();
        let mut large = support::large_ready(&path);
        large
            .set_large_cell("edited", 0, 0)
            .expect("edit the overlay");
        app.add_document(large);
        app.add_document(support::untitled(CsvConfiguration::default()));
    }

    let handle = Rc::clone(&app);
    let mut harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = handle.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();

    app.borrow_mut()
        .answer_quit_prompt(QuitResponse::Save, &harness.ctx);
    harness.run();

    let app = app.borrow();
    let large = &app.documents()[0];
    assert_eq!(app.documents().len(), 2, "the cancelled quit keeps windows");
    assert!(
        !large.is_closing_after_save(),
        "the aborted quit must clear the pending close"
    );
    assert!(!large.should_close());
    assert!(!large.is_saving());
    assert!(!large.is_dirty(), "the large save itself completed");
    assert!(!app.is_quitting());
}

//! Quit attempts that are cancelled must not leave windows closing.

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tabtool_core::CsvConfiguration;
use tabtool_desktop::app::{DesktopApp, QuitResponse};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::{Theme, UiHarness};

use support::{FakeDialogs, MemoryStorage};

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

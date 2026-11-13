//! The Settings window wiring: persistence and immediate application.

mod support;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use tabtool_desktop::app::DesktopApp;
use tabtool_desktop::platform::SharedStorage;
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::harness::kittest::{NodeT, Queryable};
use tabtool_ui::{Theme, UiHarness};

use support::MemoryStorage;

fn app(path: &Path) -> Rc<RefCell<DesktopApp>> {
    let storage: SharedStorage = Arc::new(MemoryStorage::new(b""));
    Rc::new(RefCell::new(DesktopApp::with_settings_path(
        Settings::default(),
        DesktopPlatform,
        storage,
        path.to_path_buf(),
    )))
}

fn render(handle: &Rc<RefCell<DesktopApp>>) -> UiHarness<'static> {
    let app = Rc::clone(handle);
    UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    })
}

fn choose(harness: &mut UiHarness<'_>, picker: &str, option: &str) {
    harness
        .get_all_by_role_and_label(egui::accesskit::Role::ComboBox, picker)
        .last()
        .expect("the Settings window has the picker")
        .click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}

#[test]
fn a_language_change_is_saved_and_re_renders_open_documents() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let handle = app(&path);
    handle.borrow_mut().new_document();
    handle.borrow_mut().open_settings();
    let mut harness = render(&handle);
    harness.run();

    choose(&mut harness, "Language", "Deutsch");
    harness.run();
    harness.run();

    assert_eq!(handle.borrow().language(), Language::De);
    assert!(
        harness.query_by_label("Datei").is_some(),
        "open document menus switch language in the same session"
    );
    assert!(
        harness.query_by_label("Allgemein").is_some(),
        "the Settings form switches language too"
    );
    assert_eq!(
        Settings::load(&path).app_language,
        Language::De,
        "the change is persisted through Settings::save"
    );
}

#[test]
fn a_new_document_uses_the_changed_defaults_at_once() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let handle = app(&path);
    handle.borrow_mut().new_document();
    handle.borrow_mut().open_settings();
    let mut harness = render(&handle);
    harness.run();

    harness.get_by_label("New Document").click();
    harness.run();
    choose(&mut harness, "Separator", "Tab (⇥)");
    harness.run();
    harness.run();

    assert_eq!(handle.borrow().settings.new_document_separator, "\t");
    assert_eq!(
        Settings::load(&path).new_document_separator,
        "\t",
        "the default is persisted"
    );

    handle.borrow_mut().new_document();
    let separator = handle
        .borrow()
        .documents()
        .last()
        .expect("the new document exists")
        .with_document(|document| document.configuration().column_separator.clone());
    assert_eq!(
        separator, "\t",
        "the next new document uses the new default"
    );
}

#[test]
fn a_change_outside_the_window_is_mirrored_into_the_form() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let handle = app(&path);
    handle.borrow_mut().new_document();
    handle.borrow_mut().open_settings();
    let mut harness = render(&handle);
    harness.run();

    handle.borrow_mut().settings.show_format_bar = false;
    harness.run();

    assert!(!handle.borrow().settings.show_format_bar);
    let checkbox =
        harness.get_by_role_and_label(egui::accesskit::Role::CheckBox, "Show Format Bar");
    assert_eq!(
        checkbox.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False),
        "the form took the change instead of overwriting it"
    );
}

#[test]
fn grid_appearance_changes_are_saved() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("settings.json");
    let handle = app(&path);
    handle.borrow_mut().open_settings();
    let mut harness = render(&handle);
    harness.run();

    harness.get_by_label("Grid").click();
    harness.run();
    choose(&mut harness, "Row Height", "Comfortable");
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Alternating Row Colors")
        .click();
    harness.run();
    harness.run();

    let settings = Settings::load(&path);
    assert_eq!(settings.grid_row_height, 32.0);
    assert!(settings.grid_alternating_rows);
    assert_eq!(handle.borrow().settings.grid_row_height, 32.0);
}

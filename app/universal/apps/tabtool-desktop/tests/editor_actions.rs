//! Menu and native-close commands must see the text of an open cell editor.

mod support;

use std::sync::Arc;
use std::time::Instant;

use egui::{ViewportEvent, ViewportId};
use tabtool_core::CsvConfiguration;
use tabtool_desktop::documents::{DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::harness::kittest::Queryable;

use support::{FakeDialogs, MemoryStorage};

fn configuration() -> CsvConfiguration {
    CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    }
}

fn window(storage: &Arc<MemoryStorage>, dialogs: &Arc<FakeDialogs>) -> DocumentWindow {
    let storage_clone = Arc::clone(storage);
    let shared_storage: SharedStorage = storage_clone;
    let dialogs_clone = Arc::clone(dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    DocumentWindow::new(
        ViewportId::from_hash_of("editor-actions"),
        support::loaded(storage, configuration()),
        "Untitled".to_owned(),
        Language::En,
        true,
        WindowServices {
            platform: DesktopPlatform,
            dialogs: shared_dialogs,
            storage: shared_storage,
            settings: Settings::default(),
            prefers_chinese: false,
        },
    )
}

fn render(window: &DocumentWindow) -> UiHarness<'static> {
    let window = window.clone();
    UiHarness::wide(Theme::desktop_light(), move |ui| {
        window.tick(Instant::now(), ui.ctx());
        window.render(ui);
    })
}

fn edit_cell(harness: &mut UiHarness<'_>, label: &str, text: &str) {
    harness.get_by_label(label).click();
    harness.run();
    harness.get_by_label(label).click();
    harness.run();
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text(text);
    harness.run();
}

fn cell(window: &DocumentWindow, row: usize, column: usize) -> String {
    window.with_document(|document| {
        document
            .table()
            .expect("the fixture has a table")
            .cell(row, column)
    })
}

#[test]
fn undo_commits_the_editor_into_its_own_cell_first() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\nx,y\np,q\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut harness = render(&window);
    harness.run();
    edit_cell(&mut harness, "x", "edited");

    harness.get_by_label("Edit").click();
    harness.run();
    harness.get_by_label("Undo").click();
    harness.run();

    assert_eq!(cell(&window, 0, 0), "x", "Undo reverts the committed edit");
    assert_eq!(cell(&window, 1, 0), "p", "the next row keeps its own text");
    assert!(
        window.with_document(|document| document.can_redo()),
        "the editor commit must have created the undo step"
    );
}

#[test]
fn rename_column_starts_the_header_editor_from_the_table_menu() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\nx,y\np,q\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut harness = render(&window);
    harness.run();

    harness.get_by_label("a").click();
    harness.run();
    assert_eq!(
        window.grid_selection().columns,
        [0].into_iter().collect(),
        "a header click selects the column"
    );

    harness.get_by_label("Table").click();
    harness.run();
    harness.get_by_label("Rename Column").click();
    harness.run();
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text("Id");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();

    assert_eq!(
        window.with_document(|document| document.table().and_then(|table| table.headers())),
        Some(vec!["aId".to_owned(), "b".to_owned()]),
        "the Table menu must start the inline header editor"
    );
}

#[test]
fn a_modal_blocks_command_shortcuts() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\nx,y\np,q\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut harness = render(&window);
    harness.run();
    edit_cell(&mut harness, "x", "edited");
    window.request_close();
    assert!(window.has_close_prompt());
    assert!(window.has_modal(), "the prompt owns the keyboard");

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();

    assert_eq!(
        cell(&window, 0, 0),
        "xedited",
        "⌘Z must not undo under the close prompt"
    );
}

#[test]
fn a_title_bar_close_keeps_the_open_editor_text() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\nx,y\np,q\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut harness = render(&window);
    harness.run();
    edit_cell(&mut harness, "x", "edited");

    harness
        .input_mut()
        .viewports
        .entry(ViewportId::ROOT)
        .or_default()
        .events
        .push(ViewportEvent::Close);
    harness.step();

    assert!(
        window.has_close_prompt(),
        "the typed text makes the window dirty"
    );
    assert_eq!(cell(&window, 0, 0), "xedited");
    assert_eq!(cell(&window, 1, 0), "p");
}

//! Grid action dispatch through a document window and the egui clipboard.

mod support;

use std::sync::Arc;

use egui::ViewportId;
use tabtool_core::CsvConfiguration;
use tabtool_desktop::documents::{DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_ui::grid::{Clipboard, GridAction};

use support::{FakeDialogs, MemoryStorage};

#[derive(Default)]
struct FakeClipboard {
    text: Option<String>,
}

impl FakeClipboard {
    fn with(text: &str) -> Self {
        Self {
            text: Some(text.to_owned()),
        }
    }
}

impl Clipboard for FakeClipboard {
    fn copy(&mut self, text: &str) {
        self.text = Some(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        self.text.clone()
    }
}

fn window(storage: &Arc<MemoryStorage>, dialogs: &Arc<FakeDialogs>) -> DocumentWindow {
    let document = support::loaded(storage, CsvConfiguration::default());
    let storage_clone = Arc::clone(storage);
    let shared_storage: SharedStorage = storage_clone;
    let dialogs_clone = Arc::clone(dialogs);
    let shared_dialogs: SharedDialogs = dialogs_clone;
    DocumentWindow::new(
        ViewportId::from_hash_of("grid-actions"),
        document,
        "Untitled".to_owned(),
        Language::En,
        true,
        WindowServices {
            platform: DesktopPlatform,
            dialogs: shared_dialogs,
            storage: shared_storage,
        },
    )
}

#[test]
fn select_all_paste_copy_and_delete_keep_the_grid_selection_in_step() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut clipboard = FakeClipboard::with("x\ty");

    window
        .apply_grid_action(GridAction::SelectAll, &mut clipboard)
        .expect("select all");
    assert_eq!(window.grid().selection.rows, [0, 1].into_iter().collect());

    window
        .apply_grid_action(GridAction::Paste, &mut clipboard)
        .expect("paste");
    assert_eq!(window.grid().selection.rows, [2].into_iter().collect());

    window
        .apply_grid_action(GridAction::Copy, &mut clipboard)
        .expect("copy");
    assert_eq!(clipboard.text.as_deref(), Some("x\ty"));

    window
        .apply_grid_action(GridAction::DeleteSelection, &mut clipboard)
        .expect("delete");
    assert!(window.grid().selection.is_empty());
    assert!(window.is_dirty());
}

#[test]
fn structural_commands_act_on_the_grid_selection() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut clipboard = FakeClipboard::default();

    window
        .apply_grid_action(GridAction::SelectAll, &mut clipboard)
        .expect("select all");
    window
        .apply_grid_action(GridAction::AddRowBelow, &mut clipboard)
        .expect("add a row");
    assert_eq!(window.grid().selection.rows, [2].into_iter().collect());

    window
        .apply_grid_action(GridAction::AddRowBelow, &mut clipboard)
        .expect("add a row");
    assert_eq!(window.grid().selection.rows, [3].into_iter().collect());

    window
        .apply_grid_action(GridAction::Copy, &mut clipboard)
        .expect("copy");
    assert_eq!(clipboard.text.as_deref(), Some("\t"));
}

#[test]
fn copy_without_a_selection_leaves_the_clipboard_alone() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut clipboard = FakeClipboard::with("kept");

    window
        .apply_grid_action(GridAction::Copy, &mut clipboard)
        .expect("copy");

    assert_eq!(clipboard.text.as_deref(), Some("kept"));
}

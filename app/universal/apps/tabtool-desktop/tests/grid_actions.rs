//! Grid action dispatch through a document window and the egui clipboard.

mod support;

use std::sync::Arc;

use egui::ViewportId;
use tabtool_core::{CsvConfiguration, FilterCriterion, FilterOperator, RowSource};
use tabtool_desktop::documents::{DocumentWindow, WindowServices};
use tabtool_desktop::platform::{SharedDialogs, SharedStorage};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_ui::grid::{ActionOutcome, Clipboard, GridAction, GridFocus, GridSelection};

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

fn window_with(
    document: Document,
    storage: &Arc<MemoryStorage>,
    dialogs: &Arc<FakeDialogs>,
) -> DocumentWindow {
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

fn window(storage: &Arc<MemoryStorage>, dialogs: &Arc<FakeDialogs>) -> DocumentWindow {
    window_with(
        support::loaded(storage, CsvConfiguration::default()),
        storage,
        dialogs,
    )
}

/// A document whose first column equals `a` on model rows 0 and 2.
fn filtered_document(storage: &MemoryStorage) -> Document {
    let mut document = support::loaded(storage, CsvConfiguration::default());
    let column_id = document.table().expect("table").column_ids()[0];
    document.row_filter_mut().criteria = vec![FilterCriterion {
        id: 1,
        is_enabled: true,
        column_id: Some(column_id),
        op: FilterOperator::Equals,
        value: "a".to_owned(),
    }];
    document.apply_all_filters();
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    document
}

fn selection(rows: impl IntoIterator<Item = usize>) -> GridSelection {
    GridSelection {
        rows: rows.into_iter().collect(),
        columns: Default::default(),
    }
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
    assert_eq!(window.grid_selection().rows, [0, 1].into_iter().collect());

    window
        .apply_grid_action(GridAction::Paste, &mut clipboard)
        .expect("paste");
    assert_eq!(window.grid_selection().rows, [2].into_iter().collect());

    window
        .apply_grid_action(GridAction::Copy, &mut clipboard)
        .expect("copy");
    assert_eq!(clipboard.text.as_deref(), Some("x\ty"));

    window
        .apply_grid_action(GridAction::DeleteSelection, &mut clipboard)
        .expect("delete");
    assert!(window.grid_selection().is_empty());
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
    assert_eq!(window.grid_selection().rows, [2].into_iter().collect());

    window
        .apply_grid_action(GridAction::AddRowBelow, &mut clipboard)
        .expect("add a row");
    assert_eq!(window.grid_selection().rows, [3].into_iter().collect());

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

#[test]
fn filtered_display_rows_map_to_model_rows_for_edits_and_copy() {
    let storage = Arc::new(MemoryStorage::new(b"a,1\nb,2\na,3\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window_with(filtered_document(&storage), &storage, &dialogs);
    let mut clipboard = FakeClipboard::default();
    window.set_grid_selection(selection([1]));
    window.set_grid_focus(Some(GridFocus::new(1, 1)));

    let outcome = window
        .apply_grid_action(
            GridAction::CommitCell {
                row: 1,
                column: 1,
                text: "edited".to_owned(),
            },
            &mut clipboard,
        )
        .expect("edit the visible row");

    assert_eq!(outcome, ActionOutcome::Applied);
    // The visible row is display row 1 (model row 2); the selection must stay
    // on that display row, not jump to model index 2.
    assert_eq!(window.grid_selection().rows, [1].into_iter().collect());
    assert_eq!(window.grid_focus(), Some(GridFocus::new(1, 1)));

    window.set_grid_selection(selection([0, 1]));
    window
        .apply_grid_action(GridAction::Copy, &mut clipboard)
        .expect("copy the visible rows");

    assert_eq!(
        clipboard.text.as_deref(),
        Some("a\t1\na\tedited"),
        "the edit must land on model row 2, not hidden model row 1"
    );
}

#[test]
fn a_row_move_remaps_the_grid_focus() {
    let storage = Arc::new(MemoryStorage::new(b"a\nb\nc\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut clipboard = FakeClipboard::default();
    window.set_grid_selection(selection([0]));
    window.set_grid_focus(Some(GridFocus::new(0, 0)));

    window
        .apply_grid_action(
            GridAction::MoveRows {
                from: [0].into_iter().collect(),
                to: 3,
            },
            &mut clipboard,
        )
        .expect("move the row");

    assert_eq!(window.grid_selection().rows, [2].into_iter().collect());
    assert_eq!(
        window.grid_focus(),
        Some(GridFocus::new(2, 0)),
        "focus must follow the moved row so Return edits the same row"
    );
}

#[test]
fn a_menu_paste_reports_when_the_platform_text_is_missing() {
    let storage = Arc::new(MemoryStorage::new(b"a,b\n1,2\n"));
    let dialogs = FakeDialogs::cancelled();
    let window = window(&storage, &dialogs);
    let mut clipboard = FakeClipboard::default();

    let outcome = window
        .apply_grid_action(GridAction::Paste, &mut clipboard)
        .expect("ask for the paste text");
    assert_eq!(outcome, ActionOutcome::NeedsPasteText);
    assert!(window.grid_selection().is_empty());

    let mut clipboard = FakeClipboard::with("x\ty");
    let outcome = window
        .apply_grid_action(GridAction::Paste, &mut clipboard)
        .expect("paste");
    assert_eq!(outcome, ActionOutcome::Applied);
    assert_eq!(window.grid_selection().rows, [2].into_iter().collect());
}

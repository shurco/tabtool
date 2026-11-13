//! Menu tree, enablement, shortcuts and localized labels.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use tabtool_core::{CsvConfiguration, Locale, SortDirection};
use tabtool_desktop::menu::{Capabilities, MenuAction, MenuTree, menus};
use tabtool_desktop::shortcuts;
use tabtool_i18n::{Language, tr};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::widgets::text_field;
use tabtool_ui::{Theme, UiHarness};

use support::MemoryStorage;

const EN: Language = Language::En;

fn english_tree(document: &tabtool_document::Document) -> MenuTree {
    menus(EN, &Capabilities::of(document), true, false)
}

fn entry(tree: &MenuTree, action: MenuAction) -> &tabtool_desktop::menu::MenuEntry {
    tree.entry(action)
        .unwrap_or_else(|| panic!("{action:?} is missing from the menu"))
}

fn enabled(tree: &MenuTree, action: MenuAction) -> bool {
    entry(tree, action).enabled
}

fn ready_document() -> (support::MemoryStorage, tabtool_document::Document) {
    let storage = MemoryStorage::new(b"a,b,c\n1,2,3\n4,5,6\n");
    let document = support::loaded(&storage, CsvConfiguration::default());
    (storage, document)
}

#[test]
fn menu_groups_and_item_order_match_the_apple_commands() {
    let (_storage, document) = ready_document();
    let tree = english_tree(&document);

    let labels: Vec<&str> = tree
        .menus
        .iter()
        .map(|group| group.label.as_str())
        .collect();
    assert_eq!(labels, ["TabTool", "File", "Edit", "Table", "View", "Help"]);

    let actions = |label: &str| -> Vec<MenuAction> {
        tree.menus
            .iter()
            .find(|group| group.label == label)
            .expect("menu group")
            .entries
            .iter()
            .map(|entry| entry.action)
            .collect()
    };

    assert_eq!(
        actions("File"),
        [
            MenuAction::New,
            MenuAction::Open,
            MenuAction::OpenRecent,
            MenuAction::Close,
            MenuAction::Save,
            MenuAction::SaveAs,
            MenuAction::RevertToSaved,
            MenuAction::Export,
        ]
    );
    assert_eq!(
        actions("Edit"),
        [
            MenuAction::Undo,
            MenuAction::Redo,
            MenuAction::Cut,
            MenuAction::Copy,
            MenuAction::Paste,
            MenuAction::DeleteSelection,
            MenuAction::SelectAll,
        ]
    );
    assert_eq!(
        actions("Table"),
        [
            MenuAction::AddRowAbove,
            MenuAction::AddRowBelow,
            MenuAction::AddColumnLeft,
            MenuAction::AddColumnRight,
            MenuAction::DeleteRows,
            MenuAction::DeleteColumns,
            MenuAction::RenameColumn,
            MenuAction::SortAscending,
            MenuAction::SortDescending,
            MenuAction::RemoveSort,
            MenuAction::MoveRowUp,
            MenuAction::MoveRowDown,
            MenuAction::MoveColumnLeft,
            MenuAction::MoveColumnRight,
        ]
    );
    assert_eq!(
        actions("View"),
        [MenuAction::ToggleFormatBar, MenuAction::ToggleFilter]
    );
    assert_eq!(actions("Help"), [MenuAction::Website, MenuAction::License]);
    assert_eq!(
        actions("TabTool"),
        [MenuAction::About, MenuAction::Settings, MenuAction::Quit]
    );
}

#[test]
fn item_labels_are_the_apple_commands() {
    let (_storage, document) = ready_document();
    let tree = english_tree(&document);

    assert_eq!(entry(&tree, MenuAction::New).label, "New");
    assert_eq!(entry(&tree, MenuAction::Open).label, "Open…");
    assert_eq!(entry(&tree, MenuAction::OpenRecent).label, "Open Recent");
    assert_eq!(entry(&tree, MenuAction::SaveAs).label, "Save As…");
    assert_eq!(
        entry(&tree, MenuAction::RevertToSaved).label,
        "Revert to Saved"
    );
    assert_eq!(entry(&tree, MenuAction::Export).label, "Export…");
    assert_eq!(entry(&tree, MenuAction::DeleteSelection).label, "Delete");
    assert_eq!(
        entry(&tree, MenuAction::RenameColumn).label,
        "Rename Column"
    );
    assert_eq!(
        entry(&tree, MenuAction::ToggleFormatBar).label,
        "Show Format Bar"
    );
    assert_eq!(entry(&tree, MenuAction::License).label, "MIT License");
}

#[test]
fn a_ready_file_backed_document_enables_its_commands() {
    let (_storage, document) = ready_document();
    let tree = english_tree(&document);

    assert!(enabled(&tree, MenuAction::New));
    assert!(enabled(&tree, MenuAction::Open));
    assert!(enabled(&tree, MenuAction::Close));
    assert!(enabled(&tree, MenuAction::Save));
    assert!(enabled(&tree, MenuAction::SaveAs));
    assert!(enabled(&tree, MenuAction::RevertToSaved));
    assert!(enabled(&tree, MenuAction::Export));
    assert!(enabled(&tree, MenuAction::Paste));
    assert!(enabled(&tree, MenuAction::SelectAll));
    assert!(enabled(&tree, MenuAction::AddRowAbove));
    assert!(enabled(&tree, MenuAction::AddRowBelow));
    assert!(enabled(&tree, MenuAction::AddColumnLeft));
    assert!(enabled(&tree, MenuAction::AddColumnRight));
    assert!(enabled(&tree, MenuAction::ToggleFilter));

    assert!(!enabled(&tree, MenuAction::Undo));
    assert!(!enabled(&tree, MenuAction::Redo));
    assert!(!enabled(&tree, MenuAction::Cut));
    assert!(!enabled(&tree, MenuAction::Copy));
    assert!(!enabled(&tree, MenuAction::DeleteSelection));
    assert!(!enabled(&tree, MenuAction::DeleteRows));
    assert!(!enabled(&tree, MenuAction::DeleteColumns));
    assert!(!enabled(&tree, MenuAction::RenameColumn));
    assert!(!enabled(&tree, MenuAction::SortAscending));
    assert!(!enabled(&tree, MenuAction::SortDescending));
    assert!(!enabled(&tree, MenuAction::RemoveSort));
    assert!(!enabled(&tree, MenuAction::MoveRowUp));
    assert!(!enabled(&tree, MenuAction::MoveRowDown));
    assert!(!enabled(&tree, MenuAction::MoveColumnLeft));
    assert!(!enabled(&tree, MenuAction::MoveColumnRight));
    assert!(!enabled(&tree, MenuAction::OpenRecent));
}

#[test]
fn a_loading_document_disables_editing_and_saving() {
    let document = tabtool_document::Document::new(CsvConfiguration::default());
    let tree = english_tree(&document);

    for action in [
        MenuAction::Save,
        MenuAction::SaveAs,
        MenuAction::RevertToSaved,
        MenuAction::Export,
        MenuAction::Undo,
        MenuAction::Copy,
        MenuAction::Paste,
        MenuAction::SelectAll,
        MenuAction::AddRowAbove,
        MenuAction::AddColumnLeft,
        MenuAction::RenameColumn,
        MenuAction::SortAscending,
        MenuAction::ToggleFilter,
    ] {
        assert!(!enabled(&tree, action), "{action:?} must be disabled");
    }
    assert!(enabled(&tree, MenuAction::New));
    assert!(enabled(&tree, MenuAction::Open));
    assert!(enabled(&tree, MenuAction::ToggleFormatBar));
}

#[test]
fn selection_drives_delete_sort_and_move_enablement() {
    let (_storage, mut document) = ready_document();

    document.selection_mut().columns.insert(1);
    let tree = english_tree(&document);
    assert!(enabled(&tree, MenuAction::DeleteColumns));
    assert!(enabled(&tree, MenuAction::DeleteSelection));
    assert!(enabled(&tree, MenuAction::RenameColumn));
    assert!(enabled(&tree, MenuAction::SortAscending));
    assert!(enabled(&tree, MenuAction::SortDescending));
    assert!(enabled(&tree, MenuAction::MoveColumnLeft));
    assert!(enabled(&tree, MenuAction::MoveColumnRight));
    assert!(!enabled(&tree, MenuAction::DeleteRows));
    assert!(!enabled(&tree, MenuAction::MoveRowUp));

    document.selection_mut().columns.clear();
    document.selection_mut().columns.insert(0);
    let tree = english_tree(&document);
    assert!(!enabled(&tree, MenuAction::MoveColumnLeft));
    assert!(enabled(&tree, MenuAction::MoveColumnRight));

    document.selection_mut().columns.clear();
    document.selection_mut().columns.insert(2);
    let tree = english_tree(&document);
    assert!(enabled(&tree, MenuAction::MoveColumnLeft));
    assert!(!enabled(&tree, MenuAction::MoveColumnRight));

    document.selection_mut().columns.clear();
    document.selection_mut().rows.insert(0);
    let tree = english_tree(&document);
    assert!(enabled(&tree, MenuAction::DeleteRows));
    assert!(enabled(&tree, MenuAction::DeleteSelection));
    assert!(!enabled(&tree, MenuAction::MoveRowUp));
    assert!(enabled(&tree, MenuAction::MoveRowDown));
}

#[test]
fn a_sort_makes_remove_sort_available() {
    let (_storage, mut document) = ready_document();
    let locale: Locale = "en-US".parse().expect("a valid locale");

    document.sort_rows(1, SortDirection::Ascending, &locale);
    let tree = english_tree(&document);

    assert!(enabled(&tree, MenuAction::RemoveSort));
    assert!(enabled(&tree, MenuAction::SortAscending));
}

#[test]
fn the_filter_item_follows_the_bar_visibility() {
    let (_storage, mut document) = ready_document();
    let tree = english_tree(&document);
    assert_eq!(entry(&tree, MenuAction::ToggleFilter).label, "Show Filter");
    assert!(!entry(&tree, MenuAction::ToggleFilter).selected);

    document.toggle_filter_bar();
    let tree = english_tree(&document);
    assert_eq!(entry(&tree, MenuAction::ToggleFilter).label, "Hide Filter");
    assert!(entry(&tree, MenuAction::ToggleFilter).selected);
}

#[test]
fn the_format_bar_item_reflects_the_setting() {
    let (_storage, document) = ready_document();
    let capabilities = Capabilities::of(&document);

    let hidden = menus(EN, &capabilities, false, false);
    assert!(!entry(&hidden, MenuAction::ToggleFormatBar).selected);

    let shown = menus(EN, &capabilities, true, false);
    assert!(entry(&shown, MenuAction::ToggleFormatBar).selected);
}

#[test]
fn open_recent_is_enabled_only_with_recent_documents() {
    let (_storage, document) = ready_document();
    let capabilities = Capabilities::of(&document);

    let empty = menus(EN, &capabilities, true, false);
    assert!(!enabled(&empty, MenuAction::OpenRecent));

    let recent = menus(EN, &capabilities, true, true);
    assert!(enabled(&recent, MenuAction::OpenRecent));
}

#[test]
fn large_documents_disable_the_in_memory_commands() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("big.csv");
    std::fs::write(&path, b"a,b\n1,2\n").expect("fixture file is written");

    let mut document = tabtool_document::Document::new(CsvConfiguration::default());
    document
        .open_large(&path, None, false, 0)
        .expect("open the fixture large");
    let tree = english_tree(&document);

    assert!(!enabled(&tree, MenuAction::Save));
    assert!(!enabled(&tree, MenuAction::SaveAs));
    assert!(!enabled(&tree, MenuAction::Export));
    assert!(!enabled(&tree, MenuAction::ToggleFilter));
    assert!(!enabled(&tree, MenuAction::AddRowAbove));
    assert!(!enabled(&tree, MenuAction::SelectAll));
    assert!(enabled(&tree, MenuAction::Close));
    assert!(enabled(&tree, MenuAction::RevertToSaved));
}

#[test]
fn an_untitled_document_cannot_save_in_place_but_can_save_as() {
    let document = support::untitled(CsvConfiguration::default());
    let tree = english_tree(&document);

    assert!(!enabled(&tree, MenuAction::Save));
    assert!(enabled(&tree, MenuAction::SaveAs));
    assert!(!enabled(&tree, MenuAction::RevertToSaved));
}

#[test]
fn labels_follow_the_selected_language() {
    let (_storage, document) = ready_document();
    let capabilities = Capabilities::of(&document);
    let tree = menus(Language::Fr, &capabilities, true, false);

    assert!(
        tree.group("Fichier").is_some(),
        "the File overlay is French"
    );
    assert_eq!(entry(&tree, MenuAction::New).label, "Nouveau");
    assert_eq!(
        entry(&tree, MenuAction::Save).label,
        tr(Language::Fr, "Save")
    );
}

#[test]
fn shortcut_hints_exist_only_for_bound_commands() {
    let save = shortcuts::hint(MenuAction::Save).expect("Save has a shortcut");
    assert!(save.contains('S'));
    let save_as = shortcuts::hint(MenuAction::SaveAs).expect("Save As has a shortcut");
    assert!(save_as.contains('S'));
    assert_ne!(save, save_as);

    assert!(shortcuts::hint(MenuAction::MoveRowUp).is_some());
    assert!(shortcuts::hint(MenuAction::Open).is_some());
    assert!(shortcuts::hint(MenuAction::Quit).is_some());
    assert!(shortcuts::hint(MenuAction::AddRowAbove).is_none());
}

#[test]
fn disabled_menu_commands_ignore_their_shortcuts() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("big.csv");
    std::fs::write(&path, b"a,b\n1,2\n").expect("fixture file is written");
    let mut large = support::large_ready(&path);
    large
        .set_large_cell("edited", 0, 0)
        .expect("edit the overlay");
    large.start_large_save(None).expect("start the large save");
    assert!(matches!(
        large.large_status(),
        tabtool_document::LargeStatus::Saving(_)
    ));

    let saving_tree = menus(EN, &Capabilities::of(&large), true, false);
    assert!(
        !tabtool_desktop::menu::is_enabled(&saving_tree, MenuAction::Save),
        "Save is disabled while a large save is running"
    );

    let actions = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&actions);
    let tree = Rc::new(RefCell::new(saving_tree));
    let active_tree = Rc::clone(&tree);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(action) = shortcuts::consume_enabled(ui.ctx(), &active_tree.borrow()) {
            recorded.borrow_mut().push(action);
        }
        ui.label("Background");
    });

    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();
    assert!(
        actions.borrow().is_empty(),
        "Ctrl/Cmd+S must not save while the menu disables Save"
    );

    large.cancel_large_save();
    large.wait_for_large_save().expect("cancel the large save");
    let ready_tree = menus(EN, &Capabilities::of(&large), true, false);
    assert!(
        !tabtool_desktop::menu::is_enabled(&ready_tree, MenuAction::Export),
        "Export is disabled on a large document"
    );
    *tree.borrow_mut() = ready_tree;

    harness.key_press_modifiers(
        egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
        egui::Key::E,
    );
    harness.run();
    assert!(
        actions.borrow().is_empty(),
        "Shift+Ctrl/Cmd+E must not export a large document"
    );
}

#[test]
fn command_shortcuts_do_not_fire_while_a_text_field_has_focus() {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let value = Rc::new(RefCell::new(String::new()));
    let recorded = Rc::clone(&actions);
    let field = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(action) = shortcuts::consume(ui.ctx()) {
            recorded.borrow_mut().push(action);
        }
        text_field(ui, "Name", &mut field.borrow_mut(), "Name");
    });

    harness.run();
    harness.get_by_label("Name").click();
    harness.run();

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();

    assert!(
        actions.borrow().is_empty(),
        "typing must win over the menu shortcut"
    );
}

#[test]
fn command_shortcuts_fire_without_text_focus() {
    let actions = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&actions);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(action) = shortcuts::consume(ui.ctx()) {
            recorded.borrow_mut().push(action);
        }
        ui.label("Background");
    });

    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    harness.run();

    assert_eq!(*actions.borrow(), vec![MenuAction::Save]);
}

#[test]
fn menu_clicks_report_the_chosen_action() {
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&chosen);
    let (_storage, document) = ready_document();
    let tree = english_tree(&document);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(action) = tabtool_desktop::menu::show(ui, &tree) {
            recorded.borrow_mut().push(action);
        }
    });

    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("New").click();
    harness.run();

    assert_eq!(*chosen.borrow(), vec![MenuAction::New]);
}

#[test]
fn a_disabled_menu_item_does_not_fire() {
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&chosen);
    let capabilities = Capabilities::none();
    let tree = menus(EN, &capabilities, true, false);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(action) = tabtool_desktop::menu::show(ui, &tree) {
            recorded.borrow_mut().push(action);
        }
    });

    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Save").click();
    harness.run();

    assert!(chosen.borrow().is_empty());
    assert!(!enabled(
        &menus(EN, &capabilities, true, false),
        MenuAction::Save
    ));
}

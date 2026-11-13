//! The in-window menu tree from `AppCommands.swift`.
//!
//! The builder produces owned labels so the tree can be built once per frame
//! and handed to [`show`]. Enablement comes from [`Capabilities`]; shortcut
//! hints come from [`crate::shortcuts`].

mod action;
mod capabilities;

pub use action::MenuAction;
pub use capabilities::Capabilities;

use egui::Ui;
use tabtool_i18n::{Language, tr};
use tabtool_ui::widgets::{Menu, MenuItem, menu_bar};

use crate::shortcuts;

/// One item of a menu group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuEntry {
    /// The command the item runs.
    pub action: MenuAction,
    /// The localized label.
    pub label: String,
    /// The platform shortcut hint.
    pub shortcut: Option<String>,
    /// Whether the item can be chosen.
    pub enabled: bool,
    /// Whether the item is a checked toggle.
    pub selected: bool,
}

/// One top-level menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuGroup {
    /// The localized menu title.
    pub label: String,
    /// The items in display order.
    pub entries: Vec<MenuEntry>,
}

/// The complete menu tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuTree {
    /// The top-level menus, app menu first.
    pub menus: Vec<MenuGroup>,
}

impl MenuTree {
    /// The group with the given label.
    pub fn group(&self, label: &str) -> Option<&MenuGroup> {
        self.menus.iter().find(|group| group.label == label)
    }

    /// The first entry for `action`, across every group.
    pub fn entry(&self, action: MenuAction) -> Option<&MenuEntry> {
        self.menus
            .iter()
            .flat_map(|group| &group.entries)
            .find(|entry| entry.action == action)
    }
}

/// Builds the Apple menu tree.
///
/// `show_format_bar` is the persisted View setting; `has_recent` enables
/// File > Open Recent once the settings hold a path.
pub fn menus(
    language: Language,
    capabilities: &Capabilities,
    show_format_bar: bool,
    has_recent: bool,
) -> MenuTree {
    let entry = |action: MenuAction, enabled: bool| MenuEntry {
        action,
        label: tr(language, action_label_key(action)).to_owned(),
        shortcut: shortcuts::hint(action),
        enabled,
        selected: false,
    };
    let checked = |action: MenuAction, enabled: bool, selected: bool| MenuEntry {
        selected,
        ..entry(action, enabled)
    };

    let app_menu = MenuGroup {
        label: "TabTool".to_owned(),
        entries: vec![
            entry(MenuAction::About, true),
            entry(MenuAction::Settings, true),
            entry(MenuAction::Quit, true),
        ],
    };
    let file = MenuGroup {
        label: tr(language, "File").to_owned(),
        entries: vec![
            entry(MenuAction::New, true),
            entry(MenuAction::Open, true),
            entry(MenuAction::OpenRecent, has_recent),
            entry(MenuAction::Close, capabilities.document),
            entry(MenuAction::Save, capabilities.save),
            entry(MenuAction::SaveAs, capabilities.save_as),
            entry(MenuAction::RevertToSaved, capabilities.revert),
            entry(MenuAction::Export, capabilities.export),
        ],
    };
    let edit = MenuGroup {
        label: tr(language, "Edit").to_owned(),
        entries: vec![
            entry(MenuAction::Undo, capabilities.undo),
            entry(MenuAction::Redo, capabilities.redo),
            entry(MenuAction::Cut, capabilities.cut),
            entry(MenuAction::Copy, capabilities.copy),
            entry(MenuAction::Paste, capabilities.paste),
            entry(MenuAction::DeleteSelection, capabilities.delete),
            entry(MenuAction::SelectAll, capabilities.select_all),
        ],
    };
    let table = MenuGroup {
        label: tr(language, "Table").to_owned(),
        entries: vec![
            entry(MenuAction::AddRowAbove, capabilities.insert_rows),
            entry(MenuAction::AddRowBelow, capabilities.insert_rows),
            entry(MenuAction::AddColumnLeft, capabilities.add_column),
            entry(MenuAction::AddColumnRight, capabilities.add_column),
            entry(MenuAction::DeleteRows, capabilities.delete_rows),
            entry(MenuAction::DeleteColumns, capabilities.delete_columns),
            entry(MenuAction::RenameColumn, capabilities.rename_column),
            entry(MenuAction::SortAscending, capabilities.sort),
            entry(MenuAction::SortDescending, capabilities.sort),
            entry(MenuAction::RemoveSort, capabilities.remove_sort),
            entry(MenuAction::MoveRowUp, capabilities.move_row_up),
            entry(MenuAction::MoveRowDown, capabilities.move_row_down),
            entry(MenuAction::MoveColumnLeft, capabilities.move_column_left),
            entry(MenuAction::MoveColumnRight, capabilities.move_column_right),
        ],
    };
    let filter_label = if capabilities.filter_visible {
        "Hide Filter"
    } else {
        "Show Filter"
    };
    let view = MenuGroup {
        label: tr(language, "View").to_owned(),
        entries: vec![
            checked(MenuAction::ToggleFormatBar, true, show_format_bar),
            MenuEntry {
                action: MenuAction::ToggleFilter,
                label: tr(language, filter_label).to_owned(),
                shortcut: shortcuts::hint(MenuAction::ToggleFilter),
                enabled: capabilities.filter,
                selected: capabilities.filter_visible,
            },
        ],
    };
    let help = MenuGroup {
        label: tr(language, "Help").to_owned(),
        entries: vec![
            entry(MenuAction::Website, true),
            entry(MenuAction::License, true),
        ],
    };

    MenuTree {
        menus: vec![app_menu, file, edit, table, view, help],
    }
}

/// The catalog key for an action's visible label.
///
/// `ToggleFilter` is resolved by the builder because its label depends on the
/// current bar visibility.
fn action_label_key(action: MenuAction) -> &'static str {
    match action {
        MenuAction::About => "About TabTool",
        MenuAction::Settings => "Settings…",
        MenuAction::Quit => "Quit",
        MenuAction::New => "New",
        MenuAction::Open => "Open…",
        MenuAction::OpenRecent => "Open Recent",
        MenuAction::Close => "Close",
        MenuAction::Save => "Save",
        MenuAction::SaveAs => "Save As…",
        MenuAction::RevertToSaved => "Revert to Saved",
        MenuAction::Export => "Export…",
        MenuAction::Undo => "Undo",
        MenuAction::Redo => "Redo",
        MenuAction::Cut => "Cut",
        MenuAction::Copy => "Copy",
        MenuAction::Paste => "Paste",
        MenuAction::DeleteSelection => "Delete",
        MenuAction::SelectAll => "Select All",
        MenuAction::AddRowAbove => "Add Row Above",
        MenuAction::AddRowBelow => "Add Row Below",
        MenuAction::AddColumnLeft => "Add Column Left",
        MenuAction::AddColumnRight => "Add Column Right",
        MenuAction::DeleteRows => "Delete Row(s)",
        MenuAction::DeleteColumns => "Delete Column(s)",
        MenuAction::RenameColumn => "Rename Column",
        MenuAction::SortAscending => "Sort Ascending",
        MenuAction::SortDescending => "Sort Descending",
        MenuAction::RemoveSort => "Remove Sort",
        MenuAction::MoveRowUp => "Move Row Up",
        MenuAction::MoveRowDown => "Move Row Down",
        MenuAction::MoveColumnLeft => "Move Column Left",
        MenuAction::MoveColumnRight => "Move Column Right",
        MenuAction::ToggleFormatBar => "Show Format Bar",
        MenuAction::ToggleFilter => "Show Filter",
        MenuAction::Website => "Website",
        MenuAction::License => "MIT License",
    }
}

/// Whether the tree enables `action`.
///
/// Shortcuts use the same answer as the menu item, so a disabled command can
/// never run from the keyboard.
pub fn is_enabled(tree: &MenuTree, action: MenuAction) -> bool {
    tree.entry(action).is_some_and(|entry| entry.enabled)
}

/// Shows the menu bar and returns the chosen action.
pub fn show(ui: &mut Ui, tree: &MenuTree) -> Option<MenuAction> {
    let mut items: Vec<Vec<MenuItem<'_>>> = tree
        .menus
        .iter()
        .map(|group| {
            group
                .entries
                .iter()
                .map(|entry| {
                    let mut item = MenuItem::new(entry.action.id(), &entry.label)
                        .enabled(entry.enabled)
                        .selected(entry.selected);
                    if let Some(shortcut) = &entry.shortcut {
                        item = item.shortcut(shortcut);
                    }
                    item
                })
                .collect()
        })
        .collect();
    let menus: Vec<Menu<'_>> = tree
        .menus
        .iter()
        .zip(items.iter_mut())
        .map(|(group, entries)| Menu::new(&group.label, entries))
        .collect();

    menu_bar(ui, &menus).and_then(MenuAction::from_id)
}

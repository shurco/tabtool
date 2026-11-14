//! Context menus on headers, rows and cells.
//!
//! The menus are data first: [`header_menu`], [`row_menu`] and [`cell_menu`]
//! build an ordered list of entries, and the render pass draws that list with
//! [`show`]. The grid has no document, so the flags that depend on the document
//! arrive through [`GridMenuOptions`]; the selection-dependent enablement is
//! computed from the selection the menu was opened on.

use egui::Response;
use tabtool_core::SortDirection;
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use super::GridAction;
use super::selection::{GridElement, GridSelection};

/// The document state a context menu cannot see for itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GridMenuOptions {
    /// Rows and columns can be added, removed, sorted or renamed.
    pub structural: bool,
    /// Rows can be added, removed or moved; a filtered view locks them.
    /// Implies [`Self::structural`].
    pub row_structural: bool,
    /// Whether a sort is active and can be removed.
    pub can_remove_sort: bool,
    /// Whether the document is a large file.
    ///
    /// Select All stays disabled: the current selection type would hold one
    /// entry per row, which is not viable at millions of rows (design §14.18,
    /// approval pending).
    pub large: bool,
}

impl Default for GridMenuOptions {
    fn default() -> Self {
        Self {
            structural: true,
            row_structural: true,
            can_remove_sort: false,
            large: false,
        }
    }
}

impl GridMenuOptions {
    /// Reads the capabilities the document reports now.
    pub fn of(document: &Document) -> Self {
        let structural = document.can_edit_structure();
        Self {
            structural,
            row_structural: structural && document.can_edit_row_structure(),
            can_remove_sort: document.active_sort().is_some(),
            large: !document.can_select_all(),
        }
    }
}

/// A command a menu entry runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MenuCommand {
    /// A document command.
    Run(GridAction),
    /// Open the inline header editor for a visual column.
    StartRename {
        /// The visual column under the pointer.
        column: usize,
    },
}

/// One entry of a context menu; an empty label is a separator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuEntry {
    /// The English catalog key.
    pub label: &'static str,
    /// The command, or `None` for a separator.
    pub command: Option<MenuCommand>,
    /// Whether the entry can be chosen.
    pub enabled: bool,
}

impl MenuEntry {
    /// The entry shown for `label`.
    pub fn item(label: &'static str, command: MenuCommand, enabled: bool) -> Self {
        Self {
            label,
            command: Some(command),
            enabled,
        }
    }

    /// A separator between command groups.
    pub fn separator() -> Self {
        Self {
            label: "",
            command: None,
            enabled: false,
        }
    }

    /// Whether this entry separates command groups.
    pub fn is_separator(&self) -> bool {
        self.command.is_none()
    }

    /// The localized label.
    pub fn text(&self, language: Language) -> &'static str {
        tr(language, self.label)
    }
}

fn run(action: GridAction) -> MenuCommand {
    MenuCommand::Run(action)
}

/// The menu for a header right-click on `column`.
pub fn header_menu(
    options: &GridMenuOptions,
    column: usize,
    selection: &GridSelection,
    column_count: usize,
) -> Vec<MenuEntry> {
    let structural = options.structural;
    vec![
        MenuEntry::item(
            "Sort Ascending",
            run(GridAction::Sort {
                column,
                direction: SortDirection::Ascending,
            }),
            structural,
        ),
        MenuEntry::item(
            "Sort Descending",
            run(GridAction::Sort {
                column,
                direction: SortDirection::Descending,
            }),
            structural,
        ),
        MenuEntry::item(
            "Remove Sort",
            run(GridAction::RemoveSort),
            options.can_remove_sort,
        ),
        MenuEntry::separator(),
        MenuEntry::item(
            "Add Column Left",
            run(GridAction::AddColumnLeft),
            structural,
        ),
        MenuEntry::item(
            "Add Column Right",
            run(GridAction::AddColumnRight),
            structural,
        ),
        MenuEntry::separator(),
        MenuEntry::item(
            "Delete Column(s)",
            run(GridAction::DeleteSelection),
            structural && !selection.columns.is_empty() && column_count > 1,
        ),
        MenuEntry::separator(),
        MenuEntry::item(
            "Rename Column",
            MenuCommand::StartRename { column },
            structural && column < column_count,
        ),
    ]
}

/// The menu for a row right-click.
pub fn row_menu(options: &GridMenuOptions, selection: &GridSelection) -> Vec<MenuEntry> {
    let editable_rows = options.structural && options.row_structural;
    vec![
        MenuEntry::item("Add Row Above", run(GridAction::AddRowAbove), editable_rows),
        MenuEntry::item("Add Row Below", run(GridAction::AddRowBelow), editable_rows),
        MenuEntry::separator(),
        MenuEntry::item(
            "Delete Row(s)",
            run(GridAction::DeleteSelection),
            editable_rows && !selection.rows.is_empty(),
        ),
    ]
}

/// The menu for a cell right-click.
pub fn cell_menu(
    options: &GridMenuOptions,
    selection: &GridSelection,
    row_count: usize,
    column_count: usize,
) -> Vec<MenuEntry> {
    let can_copy = selection.rows.iter().any(|row| *row < row_count)
        || selection
            .columns
            .iter()
            .any(|column| *column < column_count);
    vec![
        MenuEntry::item("Copy", run(GridAction::Copy), can_copy),
        MenuEntry::item(
            "Paste",
            run(GridAction::Paste),
            options.structural && options.row_structural,
        ),
        MenuEntry::separator(),
        MenuEntry::item(
            "Select All",
            run(GridAction::SelectAll),
            row_count > 0 && !options.large,
        ),
    ]
}

/// The selection a right-click acts on.
///
/// A click inside an existing multi-selection keeps it; a click elsewhere
/// replaces it with the clicked row or column.
pub(crate) fn context_selection(
    element: GridElement,
    row: usize,
    column: usize,
    selection: &GridSelection,
) -> GridSelection {
    match element {
        GridElement::ColumnHeader => {
            if selection.columns.contains(&column) {
                selection.clone()
            } else {
                GridSelection {
                    rows: Default::default(),
                    columns: [column].into_iter().collect(),
                }
            }
        }
        GridElement::Cell | GridElement::RowNumber => {
            if selection.rows.contains(&row) {
                selection.clone()
            } else {
                GridSelection {
                    rows: [row].into_iter().collect(),
                    columns: Default::default(),
                }
            }
        }
        GridElement::Corner => selection.clone(),
    }
}

/// Draws `entries` for `response` and returns the chosen command.
///
/// Must be called every frame: egui owns the popup's open state.
pub(crate) fn show(
    response: &Response,
    entries: &[MenuEntry],
    language: Language,
    long_pressed: bool,
) -> Option<MenuCommand> {
    let mut chosen = None;
    let popup = if long_pressed {
        // A long touch reports `clicked`, not `secondary_clicked`, so the
        // standard context-menu builder would immediately close it. Open the
        // menu explicitly for that one frame; later frames keep its state.
        egui::Popup::menu(response)
            .open_memory(Some(egui::SetOpenCommand::Bool(true)))
            .at_pointer_fixed()
    } else {
        egui::Popup::context_menu(response)
    };
    popup.show(|ui| {
        for entry in entries {
            if entry.is_separator() {
                ui.separator();
                continue;
            }
            if ui
                .add_enabled(entry.enabled, egui::Button::new(entry.text(language)))
                .clicked()
            {
                chosen = entry.command.clone();
                ui.close();
            }
        }
    });
    chosen
}

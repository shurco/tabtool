//! Stable identifiers for the menu commands.

/// One command from the Apple menu tree.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MenuAction {
    /// Show the About window.
    About,
    /// Show the Settings window.
    Settings,
    /// Quit the application.
    Quit,
    /// Create a new untitled document.
    New,
    /// Open files.
    Open,
    /// Open a recent document.
    OpenRecent,
    /// Close the focused document.
    Close,
    /// Save in place.
    Save,
    /// Save with a new destination.
    SaveAs,
    /// Revert to the saved file.
    RevertToSaved,
    /// Export a copy.
    Export,
    /// Undo the last change.
    Undo,
    /// Redo the last undone change.
    Redo,
    /// Cut the selection.
    Cut,
    /// Copy the selection.
    Copy,
    /// Paste clipboard text.
    Paste,
    /// Delete the selection.
    DeleteSelection,
    /// Select every row.
    SelectAll,
    /// Insert a row above the selection.
    AddRowAbove,
    /// Insert a row below the selection.
    AddRowBelow,
    /// Insert a column left of the selection.
    AddColumnLeft,
    /// Insert a column right of the selection.
    AddColumnRight,
    /// Delete the selected rows.
    DeleteRows,
    /// Delete the selected columns.
    DeleteColumns,
    /// Rename the selected column.
    RenameColumn,
    /// Sort the selected column ascending.
    SortAscending,
    /// Sort the selected column descending.
    SortDescending,
    /// Remove the active sort.
    RemoveSort,
    /// Move the selected rows up.
    MoveRowUp,
    /// Move the selected rows down.
    MoveRowDown,
    /// Move the selected columns left.
    MoveColumnLeft,
    /// Move the selected columns right.
    MoveColumnRight,
    /// Toggle the format bar.
    ToggleFormatBar,
    /// Show or hide the filter bar.
    ToggleFilter,
    /// Open the website.
    Website,
    /// Open the license.
    License,
}

impl MenuAction {
    /// The stable id the menu widget returns.
    pub fn id(self) -> &'static str {
        match self {
            Self::About => "app.about",
            Self::Settings => "app.settings",
            Self::Quit => "app.quit",
            Self::New => "file.new",
            Self::Open => "file.open",
            Self::OpenRecent => "file.openRecent",
            Self::Close => "file.close",
            Self::Save => "file.save",
            Self::SaveAs => "file.saveAs",
            Self::RevertToSaved => "file.revert",
            Self::Export => "file.export",
            Self::Undo => "edit.undo",
            Self::Redo => "edit.redo",
            Self::Cut => "edit.cut",
            Self::Copy => "edit.copy",
            Self::Paste => "edit.paste",
            Self::DeleteSelection => "edit.delete",
            Self::SelectAll => "edit.selectAll",
            Self::AddRowAbove => "table.addRowAbove",
            Self::AddRowBelow => "table.addRowBelow",
            Self::AddColumnLeft => "table.addColumnLeft",
            Self::AddColumnRight => "table.addColumnRight",
            Self::DeleteRows => "table.deleteRows",
            Self::DeleteColumns => "table.deleteColumns",
            Self::RenameColumn => "table.renameColumn",
            Self::SortAscending => "table.sortAscending",
            Self::SortDescending => "table.sortDescending",
            Self::RemoveSort => "table.removeSort",
            Self::MoveRowUp => "table.moveRowUp",
            Self::MoveRowDown => "table.moveRowDown",
            Self::MoveColumnLeft => "table.moveColumnLeft",
            Self::MoveColumnRight => "table.moveColumnRight",
            Self::ToggleFormatBar => "view.formatBar",
            Self::ToggleFilter => "view.filter",
            Self::Website => "help.website",
            Self::License => "help.license",
        }
    }

    /// Whether the app shell, not a document window, handles this action.
    pub fn is_app_action(self) -> bool {
        matches!(
            self,
            Self::About
                | Self::Settings
                | Self::Quit
                | Self::New
                | Self::Open
                | Self::OpenRecent
                | Self::ToggleFormatBar
                | Self::Website
                | Self::License
        )
    }

    /// The action with the given stable id.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "app.about" => Self::About,
            "app.settings" => Self::Settings,
            "app.quit" => Self::Quit,
            "file.new" => Self::New,
            "file.open" => Self::Open,
            "file.openRecent" => Self::OpenRecent,
            "file.close" => Self::Close,
            "file.save" => Self::Save,
            "file.saveAs" => Self::SaveAs,
            "file.revert" => Self::RevertToSaved,
            "file.export" => Self::Export,
            "edit.undo" => Self::Undo,
            "edit.redo" => Self::Redo,
            "edit.cut" => Self::Cut,
            "edit.copy" => Self::Copy,
            "edit.paste" => Self::Paste,
            "edit.delete" => Self::DeleteSelection,
            "edit.selectAll" => Self::SelectAll,
            "table.addRowAbove" => Self::AddRowAbove,
            "table.addRowBelow" => Self::AddRowBelow,
            "table.addColumnLeft" => Self::AddColumnLeft,
            "table.addColumnRight" => Self::AddColumnRight,
            "table.deleteRows" => Self::DeleteRows,
            "table.deleteColumns" => Self::DeleteColumns,
            "table.renameColumn" => Self::RenameColumn,
            "table.sortAscending" => Self::SortAscending,
            "table.sortDescending" => Self::SortDescending,
            "table.removeSort" => Self::RemoveSort,
            "table.moveRowUp" => Self::MoveRowUp,
            "table.moveRowDown" => Self::MoveRowDown,
            "table.moveColumnLeft" => Self::MoveColumnLeft,
            "table.moveColumnRight" => Self::MoveColumnRight,
            "view.formatBar" => Self::ToggleFormatBar,
            "view.filter" => Self::ToggleFilter,
            "help.website" => Self::Website,
            "help.license" => Self::License,
            _ => return None,
        })
    }
}

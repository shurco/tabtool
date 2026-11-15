//! The Android navigation: library, one editor, and sheets on top.
//!
//! [`AndroidNav`] wraps the shared [`NavStack`] and adds the persisted shape
//! the process-death file stores. Back handling and the dirty prompt live in
//! the app, because they need the document.

use std::path::{Path, PathBuf};

use tabtool_ui::mobile::{BackOutcome, DocumentId, NavStack, Screen, SheetKind};

/// The navigation state that survives process death.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NavSnapshot {
    /// The document open in the editor, if any.
    pub document: Option<PathBuf>,
    /// The open sheets, bottom first.
    pub sheets: Vec<SheetKind>,
}

/// The Android navigation stack.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AndroidNav {
    stack: NavStack,
}

impl AndroidNav {
    /// A stack at the library with no sheets.
    pub fn new() -> Self {
        Self::default()
    }

    /// The screen below the sheets.
    pub fn screen(&self) -> &Screen {
        self.stack.screen()
    }

    /// The path of the document in the editor.
    pub fn document(&self) -> Option<&Path> {
        match self.stack.screen() {
            Screen::Editor(document) => Some(document.path()),
            Screen::Library => None,
        }
    }

    /// The open sheets, bottom first.
    pub fn sheets(&self) -> &[SheetKind] {
        self.stack.sheets()
    }

    /// The top sheet, if any.
    pub fn sheet(&self) -> Option<SheetKind> {
        self.stack.sheet()
    }

    /// Opens `path` in the editor and closes the library's sheets.
    pub fn open_document(&mut self, path: impl Into<DocumentId>) -> bool {
        self.stack.open_document(path)
    }

    /// Presents `sheet` on top of the current screen.
    pub fn open_sheet(&mut self, sheet: SheetKind) {
        if self.stack.sheet() == Some(sheet) {
            return;
        }
        self.stack.open_sheet(sheet);
    }

    /// Closes the top sheet; returns whether one was open.
    pub fn close_sheet(&mut self) -> bool {
        self.stack.close_sheet()
    }

    /// Handles one back gesture on the navigation stack.
    pub fn back(&mut self) -> BackOutcome {
        self.stack.back()
    }

    /// Returns to the library after a document failed to open.
    pub fn document_missing(&mut self) {
        self.stack.document_missing();
    }

    /// Takes the queued notice, if any.
    pub fn take_notice(&mut self) -> Option<&'static str> {
        self.stack.take_notice()
    }

    /// The state to persist.
    pub fn snapshot(&self) -> NavSnapshot {
        NavSnapshot {
            document: self.document().map(Path::to_path_buf),
            sheets: self.sheets().to_vec(),
        }
    }
}

/// The persisted name of a sheet.
pub fn sheet_name(sheet: SheetKind) -> &'static str {
    match sheet {
        SheetKind::Format => "format",
        SheetKind::Filters => "filters",
        SheetKind::Settings => "settings",
        SheetKind::About => "about",
        SheetKind::Export => "export",
    }
}

/// The sheet a persisted name refers to.
pub fn sheet_from_name(name: &str) -> Option<SheetKind> {
    match name {
        "format" => Some(SheetKind::Format),
        "filters" => Some(SheetKind::Filters),
        "settings" => Some(SheetKind::Settings),
        "about" => Some(SheetKind::About),
        "export" => Some(SheetKind::Export),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{AndroidNav, sheet_from_name, sheet_name};
    use std::path::PathBuf;
    use tabtool_ui::mobile::{BackOutcome, Screen, SheetKind};

    #[test]
    fn back_pops_sheets_then_the_document_then_exits() {
        let mut nav = AndroidNav::new();
        nav.open_document(PathBuf::from("/docs/a.csv"));
        nav.open_sheet(SheetKind::Format);

        assert_eq!(nav.back(), BackOutcome::ClosedSheet);
        assert_eq!(nav.sheet(), None);
        assert_eq!(nav.back(), BackOutcome::ClosedDocument);
        assert!(matches!(nav.screen(), Screen::Library));
        assert_eq!(nav.back(), BackOutcome::Exit);
    }

    #[test]
    fn every_sheet_round_trips_through_its_persisted_name() {
        for sheet in [
            SheetKind::Format,
            SheetKind::Filters,
            SheetKind::Settings,
            SheetKind::About,
            SheetKind::Export,
        ] {
            assert_eq!(sheet_from_name(sheet_name(sheet)), Some(sheet));
        }
        assert_eq!(sheet_from_name("unknown"), None);
    }
}

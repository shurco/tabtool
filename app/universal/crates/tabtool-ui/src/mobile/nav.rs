//! The navigation stack: library, one editor, and sheets on top.
//!
//! Back pops one sheet, then closes the editor, and only exits at the library.
//! The app prompts for unsaved changes before it asks the stack to go back.

use std::path::{Path, PathBuf};

/// A document the library opened.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentId(PathBuf);

impl DocumentId {
    /// A document identified by its path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// The document's path.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl From<PathBuf> for DocumentId {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<&Path> for DocumentId {
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

/// The screen below the sheets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Screen {
    /// The documents library.
    Library,
    /// The editor for one document.
    Editor(DocumentId),
}

/// The sheets the mobile UI can present.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SheetKind {
    /// Format settings.
    Format,
    /// Row filters.
    Filters,
    /// App settings.
    Settings,
    /// About TabTool.
    About,
    /// Export options.
    Export,
}

/// What a back gesture did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackOutcome {
    /// A sheet was closed.
    ClosedSheet,
    /// The editor was closed and the library is showing.
    ClosedDocument,
    /// The library is showing; the app exits.
    Exit,
}

/// The notice shown when a document cannot be opened.
pub const MISSING_DOCUMENT_NOTICE: &str = "The file could not be opened";

/// The library/editor stack with its sheets.
#[derive(Clone, Debug, PartialEq)]
pub struct NavStack {
    screen: Screen,
    sheets: Vec<SheetKind>,
    notice: Option<&'static str>,
}

impl Default for NavStack {
    fn default() -> Self {
        Self::new()
    }
}

impl NavStack {
    /// A stack at the library with no sheets.
    pub fn new() -> Self {
        Self {
            screen: Screen::Library,
            sheets: Vec::new(),
            notice: None,
        }
    }

    /// The screen below the sheets.
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// The path of the document in the editor.
    pub fn document(&self) -> Option<&Path> {
        match &self.screen {
            Screen::Editor(document) => Some(document.path()),
            Screen::Library => None,
        }
    }

    /// The open sheets, bottom first.
    pub fn sheets(&self) -> &[SheetKind] {
        &self.sheets
    }

    /// The top sheet, if any.
    pub fn sheet(&self) -> Option<SheetKind> {
        self.sheets.last().copied()
    }

    /// Opens `document` in the editor.
    ///
    /// The library's sheets close and a document already in the editor is not
    /// pushed again. Returns whether the stack changed.
    pub fn open_document(&mut self, document: impl Into<DocumentId>) -> bool {
        let document = document.into();
        if self.screen == Screen::Editor(document.clone()) && self.sheets.is_empty() {
            return false;
        }
        self.screen = Screen::Editor(document);
        self.sheets.clear();
        true
    }

    /// Presents `sheet` on top of the current screen.
    pub fn open_sheet(&mut self, sheet: SheetKind) {
        self.sheets.push(sheet);
    }

    /// Closes the top sheet; returns whether one was open.
    pub fn close_sheet(&mut self) -> bool {
        self.sheets.pop().is_some()
    }

    /// Handles one back gesture.
    pub fn back(&mut self) -> BackOutcome {
        if self.sheets.pop().is_some() {
            return BackOutcome::ClosedSheet;
        }
        match self.screen {
            Screen::Editor(_) => {
                self.screen = Screen::Library;
                BackOutcome::ClosedDocument
            }
            Screen::Library => BackOutcome::Exit,
        }
    }

    /// Returns to the library after a document failed to open.
    pub fn document_missing(&mut self) {
        self.screen = Screen::Library;
        self.sheets.clear();
        self.notice = Some(MISSING_DOCUMENT_NOTICE);
    }

    /// Takes the queued notice, if any.
    pub fn take_notice(&mut self) -> Option<&'static str> {
        self.notice.take()
    }
}

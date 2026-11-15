//! The Android navigation: library, one editor, and sheets on top.
//!
//! The shared [`NavStack`] drives navigation; [`snapshot`] gives it the
//! persisted shape the process-death file stores. Back handling and the dirty
//! prompt live in the app, because they need the document.

use std::path::{Path, PathBuf};

use tabtool_ui::mobile::{NavStack, SheetKind};

/// The navigation state that survives process death.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NavSnapshot {
    /// The document open in the editor, if any.
    pub document: Option<PathBuf>,
    /// The open sheets, bottom first.
    pub sheets: Vec<SheetKind>,
}

/// The state of `nav` to persist.
pub fn snapshot(nav: &NavStack) -> NavSnapshot {
    NavSnapshot {
        document: nav.document().map(Path::to_path_buf),
        sheets: nav.sheets().to_vec(),
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
    use super::{sheet_from_name, sheet_name};
    use std::path::{Path, PathBuf};
    use tabtool_ui::mobile::{BackOutcome, NavStack, Screen, SheetKind};

    #[test]
    fn back_pops_sheets_then_the_document_then_exits() {
        let mut nav = NavStack::new();
        nav.open_document(PathBuf::from("/docs/a.csv"));
        nav.open_sheet(SheetKind::Format);
        assert_eq!(nav.document(), Some(Path::new("/docs/a.csv")));

        assert_eq!(nav.back(), BackOutcome::ClosedSheet);
        assert_eq!(nav.sheet(), None);
        assert_eq!(nav.back(), BackOutcome::ClosedDocument);
        assert!(matches!(nav.screen(), Screen::Library));
        assert_eq!(nav.document(), None);
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

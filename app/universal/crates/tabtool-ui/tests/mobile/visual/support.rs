//! Fixtures shared by the mobile visual snapshots.
//!
//! The states are rendered through the same public views the app composes:
//! the library, the touch editor and the five sheets. Image comparison is
//! opt-in per machine because system fonts and the GPU rasterizer differ.

use core::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use tabtool_document::{Document, StdStorage};
use tabtool_i18n::Language;
use tabtool_settings::Settings;
use tabtool_ui::Theme;
use tabtool_ui::UiHarness;
use tabtool_ui::mobile::{
    DocumentFiles, ExportOutcome, ExportSheet, FiltersSheet, FoldRegion, FormatSheet, LibraryClock,
    LibraryError, LibraryErrors, LibraryView, MobileMetrics, SheetStyle, about_sheet,
    settings_sheet,
};

use crate::support::{FakeTouchInput, en_locale};

/// The phone viewport the app targets.
pub const PHONE: [f32; 2] = [360.0, 800.0];

/// The tablet viewport: the same mobile layout with wider sheets.
pub const TABLET: [f32; 2] = [800.0, 1280.0];

/// A foldable phone closed: the outer display.
pub const FOLD_OUTER: [f32; 2] = [411.0, 797.0];

/// A foldable phone open: the inner display.
pub const FOLD_INNER: [f32; 2] = [841.0, 701.0];

/// The active vertical hinge of [`FOLD_INNER`], in screen points.
pub const HINGE: FoldRegion = FoldRegion::new(420.5, 32.0, 44.0);

/// A sheet style at `metrics` in English.
pub fn sheet_style(metrics: MobileMetrics) -> SheetStyle {
    SheetStyle::new(metrics, Language::En)
}

/// A documents folder the library can list.
#[derive(Clone)]
pub struct VisualFiles {
    directory: Rc<PathBuf>,
}

impl VisualFiles {
    /// An empty documents folder.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: Rc::new(directory.into()),
        }
    }

    /// Writes a fixture document and pins its modification time.
    pub fn write(&self, name: &str, contents: &str, modified: u64) -> PathBuf {
        let path = self.directory.join(name);
        std::fs::write(&path, contents).expect("the fixture is written");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("the fixture is writable");
        file.set_times(
            std::fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(modified)),
        )
        .expect("the fixture date is set");
        path
    }
}

fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["csv", "tsv", "txt", "tab", "dat"]
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

impl DocumentFiles for VisualFiles {
    fn documents_dir(&self) -> PathBuf {
        (*self.directory).clone()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(self.directory.as_path()) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && supported(path))
            .collect()
    }

    fn existing_names(&self) -> Vec<String> {
        std::fs::read_dir(self.directory.as_path())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }

    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.directory.join(name);
        std::fs::write(&path, data)?;
        Ok(path)
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        None
    }

    fn export_copy(&self, _source: &Path, _suggested_name: &str) -> ExportOutcome {
        ExportOutcome::Copied
    }

    fn delete(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }
}

/// The platform failures the library waits for.
#[derive(Clone, Default)]
pub struct VisualErrors {
    pending: Rc<RefCell<Option<LibraryError>>>,
}

impl VisualErrors {
    /// Queues `error` for the next library frame.
    pub fn push(&self, error: LibraryError) {
        self.pending.borrow_mut().replace(error);
    }
}

impl LibraryErrors for VisualErrors {
    fn take_error(&self) -> Option<LibraryError> {
        self.pending.borrow_mut().take()
    }
}

/// A library harness with an empty or populated documents folder.
pub fn library_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    files: VisualFiles,
    errors: VisualErrors,
) -> UiHarness<'static> {
    let mut view = LibraryView::new(
        files,
        errors,
        tabtool_ui::mobile::SystemMetadata,
        en_locale(),
        LibraryClock::default(),
        metrics,
        Language::En,
    );
    let mut input = FakeTouchInput::default();
    UiHarness::new(theme, size, move |ui| {
        let _ = view.show(ui, &mut input);
    })
}

/// The Format sheet over `document`.
pub fn format_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    document: Document,
) -> UiHarness<'static> {
    let document = Rc::new(RefCell::new(document));
    let sheet = Rc::new(RefCell::new(FormatSheet::new()));
    let style = sheet_style(metrics);
    UiHarness::new(theme, size, move |ui| {
        let mut document = document.borrow_mut();
        let _ = sheet
            .borrow_mut()
            .show(ui, style, &mut document, &StdStorage, false);
    })
}

/// The Filters sheet over `document`.
pub fn filters_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    document: Document,
) -> UiHarness<'static> {
    let document = Rc::new(RefCell::new(document));
    let sheet = Rc::new(RefCell::new(FiltersSheet::new()));
    let locale = en_locale();
    let style = sheet_style(metrics);
    let mut input = FakeTouchInput::default();
    UiHarness::new(theme, size, move |ui| {
        let mut document = document.borrow_mut();
        let _ = sheet
            .borrow_mut()
            .show(ui, style, &mut document, &mut input, &locale);
    })
}

/// The Settings sheet with the app defaults.
pub fn settings_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    settings: Settings,
) -> UiHarness<'static> {
    let settings = Rc::new(RefCell::new(settings));
    let style = sheet_style(metrics);
    UiHarness::new(theme, size, move |ui| {
        let _ = settings_sheet(ui, style, &mut settings.borrow_mut());
    })
}

/// The About sheet at the current version.
pub fn about_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    version: &str,
) -> UiHarness<'static> {
    let version = version.to_owned();
    let style = sheet_style(metrics);
    UiHarness::new(theme, size, move |ui| {
        let _ = about_sheet(ui, style, &version, "MIT License");
    })
}

/// The Export sheet seeded from `document`.
pub fn export_harness(
    theme: Theme,
    size: [f32; 2],
    metrics: MobileMetrics,
    document: Document,
    files: VisualFiles,
) -> UiHarness<'static> {
    let sheet = Rc::new(RefCell::new(ExportSheet::new(&document)));
    let document = Rc::new(RefCell::new(document));
    let style = sheet_style(metrics);
    UiHarness::new(theme, size, move |ui| {
        let document = document.borrow();
        let _ = sheet.borrow_mut().show(ui, style, &document, &files);
    })
}

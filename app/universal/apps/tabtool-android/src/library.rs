//! The Android library seams and its locale helpers.
//!
//! The library view wants owned seams, so these adapters hold the shared
//! platform host and forward every document operation. The app keeps the view
//! across frames to preserve an open delete prompt or rename field.

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use tabtool_core::Locale;
use tabtool_i18n::Language;
use tabtool_settings::Settings;
use tabtool_ui::mobile::{
    DocumentFiles, DocumentMetadata, ExportOutcome, FileMetadata, LibraryClock, LibraryError,
    LibraryErrors, LibraryView, MobileMetrics,
};

use crate::app::Host;

/// The library's file operations.
pub struct AppFiles {
    host: Rc<dyn Host>,
}

impl AppFiles {
    /// A seam over `host`.
    pub fn new(host: Rc<dyn Host>) -> Self {
        Self { host }
    }
}

impl DocumentFiles for AppFiles {
    fn documents_dir(&self) -> PathBuf {
        self.host.documents_dir()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        self.host.list_documents()
    }

    fn existing_names(&self) -> Vec<String> {
        self.host.existing_names()
    }

    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        self.host.create_document(name, data)
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        self.host.import_from_picker()
    }

    fn import_in_progress(&self) -> bool {
        self.host.import_in_progress()
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> ExportOutcome {
        self.host.export_copy(source, suggested_name)
    }

    fn export_in_progress(&self) -> bool {
        self.host.export_in_progress()
    }

    fn delete(&self, path: &Path) -> io::Result<()> {
        self.host.delete(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.host.rename(from, to)
    }
}

/// The platform's queued failures.
pub struct AppErrors {
    host: Rc<dyn Host>,
}

impl AppErrors {
    /// A seam over `host`.
    pub fn new(host: Rc<dyn Host>) -> Self {
        Self { host }
    }
}

impl LibraryErrors for AppErrors {
    fn take_error(&self) -> Option<LibraryError> {
        self.host.take_error()
    }
}

/// The live file metadata.
pub struct AppMetadata {
    host: Rc<dyn Host>,
}

impl AppMetadata {
    /// A seam over `host`.
    pub fn new(host: Rc<dyn Host>) -> Self {
        Self { host }
    }
}

impl FileMetadata for AppMetadata {
    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata> {
        self.host.metadata(path)
    }
}

/// A library view over `host` with the current settings and layout.
pub fn new_view(
    host: Rc<dyn Host>,
    settings: &Settings,
    locale: Locale,
    clock: LibraryClock,
    metrics: MobileMetrics,
    language: Language,
) -> LibraryView<'static> {
    LibraryView::new(
        AppFiles::new(Rc::clone(&host)),
        AppErrors::new(Rc::clone(&host)),
        AppMetadata::new(host),
        locale,
        clock,
        metrics,
        language,
    )
    .new_document_configuration(settings.new_document_configuration())
}

/// The system locale, or English when the tag is not a valid ICU locale.
pub fn parse_locale(tag: &str) -> Locale {
    let tag = tag.replace('_', "-");
    Locale::try_from_str(&tag)
        .or_else(|_| Locale::try_from_str("en"))
        .expect("English is a valid ICU locale")
}

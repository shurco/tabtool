//! The Documents library: the private folder's table files, with creation,
//! import, rename and delete.
//!
//! The screen never loads document content. Opening a row only returns the
//! path; the app owns the editor and the navigation that pushes it. The
//! library depends on the platform through the neutral [`DocumentFiles`]
//! seam, so `tabtool-ui` stays free of `tabtool-platform`.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use egui::Rect;
use tabtool_core::{CsvConfiguration, Locale, has_table_extension};
use tabtool_document::Document;
use tabtool_i18n::{ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE, Language, LocalizedError};

use super::MobileMetrics;

mod files;
mod format;
mod names;
mod screen;

pub use files::{DocumentFiles, DocumentMetadata, ExportOutcome};
pub use format::{LibraryClock, format_modified, format_size};
pub use names::{available_name, new_document_name};

/// The default extension of a new document.
const NEW_DOCUMENT_EXTENSION: &str = "csv";

/// The order the library shows documents in.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LibrarySort {
    /// Most recently modified first, then by name.
    #[default]
    NewestFirst,
}

/// A table file in the documents folder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentEntry {
    path: PathBuf,
    name: String,
    modified: SystemTime,
    size: u64,
}

impl DocumentEntry {
    /// The file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file name including its extension.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The row title: the file name without its extension.
    pub fn title(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&self.name)
    }

    /// The file size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The last modification time.
    pub fn modified(&self) -> SystemTime {
        self.modified
    }
}

/// What the screen asked the app to do after this frame.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibraryResponse {
    /// The document to open. The app pushes the editor.
    pub open: Option<PathBuf>,
    /// Show the Settings sheet.
    pub settings: bool,
    /// Show the About sheet.
    pub about: bool,
}

/// The screen state: documents, sort order and the pending prompt.
pub struct LibraryView {
    files: Rc<dyn DocumentFiles>,
    new_document: CsvConfiguration,
    locale: Locale,
    clock: LibraryClock,
    metrics: MobileMetrics,
    language: Language,
    documents: Vec<DocumentEntry>,
    loaded: bool,
    importing: bool,
    pending_open: Option<PathBuf>,
    pending_delete: Option<PathBuf>,
    renaming: Option<PathBuf>,
    rename_request: Option<(PathBuf, Rect)>,
    notice: Option<LocalizedError>,
}

impl LibraryView {
    /// A library over `files`, formatting sizes and dates for `locale` and
    /// `clock`.
    ///
    /// The seam is a shared handle, so an app can hold the view across frames
    /// while its platform services live in the same handle.
    pub fn new(
        files: Rc<dyn DocumentFiles>,
        locale: Locale,
        clock: LibraryClock,
        metrics: MobileMetrics,
        language: Language,
    ) -> Self {
        Self {
            files,
            new_document: CsvConfiguration::default(),
            locale,
            clock,
            metrics,
            language,
            documents: Vec::new(),
            loaded: false,
            importing: false,
            pending_open: None,
            pending_delete: None,
            renaming: None,
            rename_request: None,
            notice: None,
        }
    }

    /// Uses `configuration` for a new document's initial content.
    pub fn new_document_configuration(mut self, configuration: CsvConfiguration) -> Self {
        self.new_document = configuration;
        self
    }

    /// The documents currently listed, newest first.
    pub fn documents(&self) -> &[DocumentEntry] {
        &self.documents
    }

    /// The active sort order.
    pub fn sort(&self) -> LibrarySort {
        LibrarySort::NewestFirst
    }

    /// Re-reads the documents folder and sorts it newest first.
    pub fn refresh(&mut self) {
        let mut documents = Vec::new();
        for path in self.files.list_documents() {
            if !is_library_file(&path) {
                continue;
            }
            let Ok(metadata) = self.files.metadata(&path) else {
                continue;
            };
            let Some(name) = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            documents.push(DocumentEntry {
                path,
                name,
                modified: metadata.modified,
                size: metadata.size,
            });
        }
        sort_documents(&mut documents);
        self.documents = documents;
        self.loaded = true;
    }

    /// Creates a new document and returns its path.
    pub fn create_document(&mut self) -> Result<PathBuf, LocalizedError> {
        let name = new_document_name(self.files.existing_names(), NEW_DOCUMENT_EXTENSION);
        let data = new_document_data(&self.new_document)?;
        let path = self
            .files
            .create_document(&name, &data)
            .map_err(|error| LocalizedError::new(ERROR_SAVE, error.to_string()))?;
        self.refresh();
        Ok(path)
    }

    /// Asks the platform for a file and imports it under the picked name.
    ///
    /// `None` covers an open picker, a cancel and a failed copy; a failed
    /// copy is reported through [`DocumentFiles::take_error`]. A picked file the library
    /// cannot show is deleted and reported. While an asynchronous picker is
    /// open, [`LibraryView::show`] keeps polling and reports the imported
    /// document once it arrives.
    pub fn import_document(&mut self) -> Option<PathBuf> {
        if let Some(path) = self.files.import_from_picker() {
            return self.accept_import(path);
        }
        self.importing = self.files.import_in_progress();
        None
    }

    fn accept_import(&mut self, path: PathBuf) -> Option<PathBuf> {
        if !is_library_file(&path) {
            let detail = format!("unsupported import: {}", path.display());
            let _ = self.files.delete(&path);
            self.notice = Some(LocalizedError::new(ERROR_OPEN, detail));
            self.refresh();
            return None;
        }
        self.refresh();
        Some(path)
    }

    /// Deletes `path` after the caller has confirmed.
    pub fn delete_document(&mut self, path: &Path) -> Result<(), LocalizedError> {
        if let Err(error) = self.files.delete(path) {
            return Err(LocalizedError::new(ERROR_GENERIC, error.to_string()));
        }
        self.refresh();
        Ok(())
    }
}

/// Whether `path` is a visible table file the library lists.
fn is_library_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !name.starts_with('.') && has_table_extension(path)
}

fn sort_documents(documents: &mut [DocumentEntry]) {
    documents.sort_by(|lhs, rhs| {
        rhs.modified
            .cmp(&lhs.modified)
            .then_with(|| lhs.name.cmp(&rhs.name))
    });
}

fn new_document_data(configuration: &CsvConfiguration) -> Result<Vec<u8>, LocalizedError> {
    let mut document = Document::new(configuration.clone());
    document
        .new_document(configuration.clone())
        .map_err(|error| LocalizedError::new(ERROR_SAVE, format!("{error:?}")))?;
    document
        .encoded_data()
        .map_err(|error| LocalizedError::new(ERROR_SAVE, format!("{error:?}")))
}

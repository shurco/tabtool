//! The Documents library: the private folder's table files, with creation,
//! import, rename and delete.
//!
//! The screen never loads document content. Opening a row only returns the
//! path; the app owns the editor and the navigation that pushes it. The
//! library depends on the platform through the neutral [`DocumentFiles`],
//! [`LibraryErrors`] and [`FileMetadata`] seams, so `tabtool-ui` stays free of
//! `tabtool-platform`.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use egui::Rect;
use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::Document;
use tabtool_i18n::Language;

use super::MobileMetrics;

mod format;
mod screen;

pub use format::{LibraryClock, format_modified, format_size};

/// The extensions the library lists.
///
/// The list matches the platform crate's `TABLE_EXTENSIONS`; the UI crate
/// cannot import the platform crate, so the two lists are kept in step by
/// hand.
const TABLE_EXTENSIONS: [&str; 5] = ["csv", "tsv", "txt", "tab", "dat"];

/// The base name of a new document.
const NEW_DOCUMENT_BASE: &str = "Untitled";

/// The default extension of a new document.
const NEW_DOCUMENT_EXTENSION: &str = "csv";

/// The catalog key for a platform failure with no more specific message.
pub const ERROR_GENERIC: &str = "Something Went Wrong";

/// The catalog key for a document that could not be opened.
pub const ERROR_OPEN: &str = "The file could not be opened";

/// The catalog key for a document that could not be written.
pub const ERROR_SAVE: &str = "The file could not be saved";

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

/// The metadata the library shows for a document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentMetadata {
    /// The file size in bytes.
    pub size: u64,
    /// The last modification time.
    pub modified: SystemTime,
}

/// The file metadata the library reads.
pub trait FileMetadata {
    /// Reads the metadata for `path`, or reports that it is gone.
    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata>;
}

/// The live file system's metadata.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemMetadata;

impl FileMetadata for SystemMetadata {
    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a regular file", path.display()),
            ));
        }
        Ok(DocumentMetadata {
            size: metadata.len(),
            // A file system without a modification date sorts as oldest.
            modified: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        })
    }
}

/// The app's private document store and import picker.
///
/// This mirrors the platform's `DocumentFiles` trait. The app adapts its
/// platform value to this seam, which keeps the UI crate independent of the
/// platform crate.
pub trait DocumentFiles {
    /// The app's private documents directory.
    fn documents_dir(&self) -> PathBuf;

    /// The table files in [`DocumentFiles::documents_dir`].
    fn list_documents(&self) -> Vec<PathBuf>;

    /// The names in the documents directory, including files the library
    /// does not list, for new-name collisions.
    fn existing_names(&self) -> Vec<String>;

    /// Writes a new document named `name` and returns its path.
    ///
    /// The platform creates the documents directory if needed and must not
    /// replace an existing file.
    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf>;

    /// Asks the user to pick a file and copies it into the documents
    /// directory, returning the copy.
    ///
    /// The result is one-shot: `None` means the picker is still open, the
    /// user cancelled, or the copy failed. Failures are reported separately
    /// through [`LibraryErrors`].
    fn import_from_picker(&self) -> Option<PathBuf>;

    /// Whether a picker started earlier is still open.
    ///
    /// A platform with a synchronous dialog answers `false`. The library
    /// waits while this is true and then takes the result with one last
    /// [`DocumentFiles::import_from_picker`] call.
    fn import_in_progress(&self) -> bool {
        false
    }

    /// Deletes a document.
    fn delete(&self, path: &Path) -> io::Result<()>;

    /// Renames a document.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
}

/// A platform failure the library can show as a localized banner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryError {
    /// The catalog key for the banner.
    pub message_key: &'static str,
    /// The technical cause, for logs only.
    pub detail: String,
}

impl LibraryError {
    /// Builds a failure with a catalog key and a diagnostic detail.
    pub fn new(message_key: &'static str, detail: impl Into<String>) -> Self {
        Self {
            message_key,
            detail: detail.into(),
        }
    }
}

/// The platform's queued failures, so the library can show a localized
/// banner after an asynchronous import.
pub trait LibraryErrors {
    /// Takes the next failure, if any.
    fn take_error(&self) -> Option<LibraryError>;
}

/// The screen state: documents, sort order and the pending prompt.
pub struct LibraryView<'a> {
    files: &'a dyn DocumentFiles,
    errors: &'a dyn LibraryErrors,
    metadata: &'a dyn FileMetadata,
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
    notice: Option<LibraryError>,
}

impl<'a> LibraryView<'a> {
    /// A library over `files`, formatting sizes and dates for `locale` and
    /// `clock`.
    pub fn new(
        files: &'a dyn DocumentFiles,
        errors: &'a dyn LibraryErrors,
        metadata: &'a dyn FileMetadata,
        locale: Locale,
        clock: LibraryClock,
        metrics: MobileMetrics,
        language: Language,
    ) -> Self {
        Self {
            files,
            errors,
            metadata,
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
            let Ok(metadata) = self.metadata.metadata(&path) else {
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
    pub fn create_document(&mut self) -> Result<PathBuf, LibraryError> {
        let name = new_document_name(self.files.existing_names(), NEW_DOCUMENT_EXTENSION);
        let data = new_document_data(&self.new_document)?;
        let path = self
            .files
            .create_document(&name, &data)
            .map_err(|error| LibraryError::new(ERROR_SAVE, error.to_string()))?;
        self.refresh();
        Ok(path)
    }

    /// Asks the platform for a file and imports it under the picked name.
    ///
    /// `None` covers an open picker, a cancel and a failed copy; a failed
    /// copy is reported through [`LibraryErrors`]. A picked file the library
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
            self.notice = Some(LibraryError::new(ERROR_OPEN, detail));
            self.refresh();
            return None;
        }
        self.refresh();
        Some(path)
    }

    /// Renames `path` to `proposed`, keeping the extension when it is omitted.
    ///
    /// A taken name gets " 2", " 3" before the extension instead of replacing
    /// another document.
    pub fn rename_document(
        &mut self,
        path: &Path,
        proposed: &str,
    ) -> Result<PathBuf, LibraryError> {
        let trimmed = proposed.trim();
        if trimmed.is_empty() {
            return Ok(path.to_path_buf());
        }
        if trimmed == "."
            || trimmed == ".."
            || trimmed.starts_with('.')
            || trimmed.ends_with('.')
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains(':')
        {
            return Err(LibraryError::new(
                ERROR_GENERIC,
                "the document name would hide the file or leave the folder",
            ));
        }
        let proposed_path = Path::new(trimmed);
        if proposed_path.extension().is_some() && !has_table_extension(proposed_path) {
            return Err(LibraryError::new(
                ERROR_GENERIC,
                "the document name must end in a table extension",
            ));
        }
        let current = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let extension = path.extension().and_then(|extension| extension.to_str());
        let name = if trimmed.contains('.') {
            trimmed.to_owned()
        } else {
            match extension {
                Some(extension) => format!("{trimmed}.{extension}"),
                None => trimmed.to_owned(),
            }
        };
        if name == current {
            return Ok(path.to_path_buf());
        }
        let existing = self.files.existing_names();
        if name.eq_ignore_ascii_case(current) {
            // A case-only rename keeps the same file; numbering it would
            // leave two names for one document.
            if !existing.iter().any(|existing| existing == &name) {
                let destination = path.with_file_name(&name);
                if let Err(error) = self.files.rename(path, &destination) {
                    return Err(LibraryError::new(ERROR_GENERIC, error.to_string()));
                }
                self.refresh();
                return Ok(destination);
            }
        }
        let available = available_name(&name, existing);
        let destination = path.with_file_name(available);
        if let Err(error) = self.files.rename(path, &destination) {
            return Err(LibraryError::new(ERROR_GENERIC, error.to_string()));
        }
        self.refresh();
        Ok(destination)
    }

    /// Deletes `path` after the caller has confirmed.
    pub fn delete_document(&mut self, path: &Path) -> Result<(), LibraryError> {
        if let Err(error) = self.files.delete(path) {
            return Err(LibraryError::new(ERROR_GENERIC, error.to_string()));
        }
        self.refresh();
        Ok(())
    }
}

/// The new-document name: `Untitled.csv`, then `Untitled 2.csv`, and so on.
///
/// Names are compared case-insensitively because the app's folder lives on a
/// case-insensitive volume.
pub fn new_document_name(
    existing: impl IntoIterator<Item = impl AsRef<str>>,
    extension: &str,
) -> String {
    let taken = lowercased(existing);
    let base = format!("{NEW_DOCUMENT_BASE}.{extension}");
    if !taken.contains(&base.to_lowercase()) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{NEW_DOCUMENT_BASE} {index}.{extension}");
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        index += 1;
    }
}

/// The proposed name, or the same name with " 2", " 3" before the extension.
pub fn available_name(
    proposed: &str,
    existing: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    let taken = lowercased(existing);
    if !taken.contains(&proposed.to_lowercase()) {
        return proposed.to_owned();
    }
    let path = Path::new(proposed);
    let extension = path.extension().and_then(|extension| extension.to_str());
    let base = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(proposed);
    let mut index = 2;
    loop {
        let candidate = match extension {
            Some(extension) => format!("{base} {index}.{extension}"),
            None => format!("{base} {index}"),
        };
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        index += 1;
    }
}

fn lowercased(names: impl IntoIterator<Item = impl AsRef<str>>) -> BTreeSet<String> {
    names
        .into_iter()
        .map(|name| name.as_ref().to_lowercase())
        .collect()
}

/// Whether `path` is a visible table file the library lists.
fn is_library_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !name.starts_with('.') && has_table_extension(path)
}

fn has_table_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            TABLE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

fn sort_documents(documents: &mut [DocumentEntry]) {
    documents.sort_by(|lhs, rhs| {
        rhs.modified
            .cmp(&lhs.modified)
            .then_with(|| lhs.name.cmp(&rhs.name))
    });
}

fn new_document_data(configuration: &CsvConfiguration) -> Result<Vec<u8>, LibraryError> {
    let mut document = Document::new(configuration.clone());
    document
        .new_document(configuration.clone())
        .map_err(|error| LibraryError::new(ERROR_SAVE, format!("{error:?}")))?;
    document
        .encoded_data()
        .map_err(|error| LibraryError::new(ERROR_SAVE, format!("{error:?}")))
}

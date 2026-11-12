//! The document value: table, phase, selection and save state.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable, CsvWriteIssue, RowSource};

use crate::types::{ActiveSort, DocumentNotice, DocumentPhase, GridSelection, LargeStatus};

/// One CSV document.
///
/// The value owns the table and every piece of view-independent state. Loading
/// and editing lands in later tasks; this skeleton defines the state and the
/// getters those tasks share.
pub struct Document {
    phase: DocumentPhase,
    table: Option<Arc<CsvTable>>,
    /// Placeholder for the memory-mapped state of a large file.
    ///
    /// The large-file task replaces the unit value with the mapped source,
    /// indexer and file handle.
    large: Option<()>,
    configuration: CsvConfiguration,
    selection: GridSelection,
    active_sort: Option<ActiveSort>,
    /// Placeholder for the row filter; the filter task adds the model.
    #[allow(dead_code)]
    filter: (),
    /// Placeholder for the undo history; the editing task adds the stack.
    #[allow(dead_code)]
    undo: (),
    file_path: Option<PathBuf>,
    /// Whether the loaded bytes began with a UTF-8 byte order mark.
    #[allow(dead_code)] // Snapshots read it once loading lands.
    had_utf8_bom: bool,
    revision: u64,
    saved_revision: u64,
    /// A one-shot message the window should present.
    notice: Option<DocumentNotice>,
    large_status: LargeStatus,
}

impl Document {
    /// Creates an untitled document with a blank 1×3 table.
    ///
    /// The phase stays [`DocumentPhase::Loading`] until a load materializes
    /// file-backed content.
    pub fn new(configuration: CsvConfiguration) -> Self {
        Self {
            phase: DocumentPhase::Loading,
            // Task 2 reconciles this placeholder with the load path; the
            // untitled table exists now so the defaults are observable.
            table: Some(Arc::new(CsvTable::new_document(1, 3))),
            large: None,
            configuration,
            selection: GridSelection::default(),
            active_sort: None,
            filter: (),
            undo: (),
            file_path: None,
            had_utf8_bom: false,
            revision: 0,
            // Task 7 reconciles revisions with saves; an untitled document
            // reports clean until then.
            saved_revision: 0,
            notice: None,
            large_status: LargeStatus::None,
        }
    }

    /// The in-memory table, or `None` while no table is loaded.
    pub fn table(&self) -> Option<&Arc<CsvTable>> {
        self.table.as_ref()
    }

    /// The format used to read and write the document.
    pub fn configuration(&self) -> &CsvConfiguration {
        &self.configuration
    }

    /// Why the document cannot currently be edited or saved.
    pub fn phase(&self) -> &DocumentPhase {
        &self.phase
    }

    /// The rows and columns selected in the grid.
    pub fn selection(&self) -> &GridSelection {
        &self.selection
    }

    /// The rows and columns selected in the grid, for mutation.
    pub fn selection_mut(&mut self) -> &mut GridSelection {
        &mut self.selection
    }

    /// The row sort currently applied, if any.
    pub fn active_sort(&self) -> Option<&ActiveSort> {
        self.active_sort.as_ref()
    }

    /// The file the document was loaded from or saved to, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Counts every content or file-binding change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether the content differs from the last saved revision.
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// The typed problem that blocks saving, when the phase reports one.
    pub fn write_issue(&self) -> Option<CsvWriteIssue> {
        match self.phase {
            DocumentPhase::NeedsAttention(issue) => Some(issue),
            _ => None,
        }
    }

    /// Where a large file stands.
    pub fn large_status(&self) -> LargeStatus {
        self.large_status
    }

    /// Whether this document opens its file through the memory-mapped path.
    pub fn is_large_mode(&self) -> bool {
        self.large.is_some()
    }

    /// Whether a table or a large-file source is loaded.
    pub fn has_content(&self) -> bool {
        self.table.is_some() || self.large.is_some()
    }

    /// The rows the grid and status bar read.
    pub fn row_source(&self) -> Option<&dyn RowSource> {
        self.table.as_deref().map(|table| table as &dyn RowSource)
    }

    /// Whether cells can be edited now: always in memory, and in large mode
    /// once the index is complete.
    pub fn can_edit_cells(&self) -> bool {
        !self.is_large_mode() || self.large_status == LargeStatus::Ready
    }

    /// The one-shot message the window should present, if any.
    ///
    /// Always `None` until the large-file tasks raise a notice; kept now so
    /// the window has one accessor for it.
    pub fn notice(&self) -> Option<DocumentNotice> {
        self.notice
    }
}

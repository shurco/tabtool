//! Public state types shared by every consumer of a document.

use std::collections::BTreeSet;

use tabtool_core::{ColumnId, CsvError, CsvWriteIssue, SortDirection};

/// Why the document cannot currently be edited or saved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentPhase {
    /// Bytes are read, format detection and parsing are still running.
    Loading,
    /// The table can be displayed, edited, and saved.
    Ready,
    /// Parsing the bytes failed; the format must be fixed before retrying.
    ReadFailed(CsvError),
    /// The file bytes could not be read at all, as distinct from a parse error.
    UnreadableFile,
    /// The table is loaded but cannot be written safely with its configuration.
    NeedsAttention(CsvWriteIssue),
}

/// Document operations that fail before or after CSV parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentError {
    /// The table is not loaded yet or the file could not be read.
    NotReady,
    /// The operation would discard unsaved edits, or it raced with another
    /// change.
    RequiresConfirmation,
    /// The file bytes are unavailable to the document.
    CannotReadFile,
    /// Serializing the table failed with a typed CSV error, such as
    /// [`CsvError::AmbiguousOutput`] or [`CsvError::UnencodableText`].
    Csv(CsvError),
}

/// The rows and columns selected in the grid.
///
/// A table selection is either a set of rows or a set of columns; the grid
/// clears the other axis when the user changes it. Commands read this state to
/// decide what they operate on.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GridSelection {
    /// Selected row indexes.
    pub rows: BTreeSet<usize>,
    /// Selected column indexes.
    pub columns: BTreeSet<usize>,
}

/// The row sort currently applied to the table.
///
/// `source_order[i]` is the pre-sort index of current row `i`, so removing the
/// sort is the inverse permutation. It stays valid while rows and column
/// identities are unchanged, so column moves keep it; every other structural
/// change clears it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveSort {
    /// Identity of the sorted column.
    pub column_id: ColumnId,
    /// Direction the column is sorted in.
    pub direction: SortDirection,
    /// Pre-sort index of every current row.
    pub source_order: Vec<usize>,
}

/// Where a large file stands: being indexed, ready, or no longer matching disk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LargeStatus {
    /// Not a large file.
    None,
    /// The row index is being built; the value is the share of the file scanned.
    Indexing(f64),
    /// The index is complete and the rows can be edited.
    Ready,
    /// The file changed on disk. `appended` is true when it only grew.
    Stale {
        /// Whether the file only grew since it was mapped.
        appended: bool,
    },
    /// The edits are being written; the value is the share of the file written.
    Saving(f64),
}

/// Something the window tells the user once, then forgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentNotice {
    /// A large-file cell edit was refused; nothing changed.
    EditRejected(CsvError),
    /// A copy would produce more cells than the large-file limit.
    CopyLimitExceeded,
}

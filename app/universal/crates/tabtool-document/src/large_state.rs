//! The memory-mapped state of a large document.

use std::sync::Arc;

use tabtool_large::{LargeCSVSource, LargeFileIndexer, MappedFile};

/// Everything a large document owns: the mapping, its row source and its indexer.
///
/// The source reads cells through the growing index and the edit overlay; the
/// indexer fills that index on its worker thread. The file is kept explicitly
/// so the document can answer staleness without going through the source.
pub struct LargeState {
    /// The rows as the grid reads them.
    pub source: Arc<LargeCSVSource>,
    /// The background scan building the row index.
    pub indexer: Arc<LargeFileIndexer>,
    /// The read-only mapping the source reads.
    pub file: Arc<MappedFile>,
}

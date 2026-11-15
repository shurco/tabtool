//! The memory-mapped state of a large document.

use std::sync::Arc;
use std::time::Duration;

use tabtool_large::{LargeCSVSource, LargeFileIndexer, MappedFile};

/// Bytes the indexer scans per step, matching the Apple default.
pub(crate) const INDEX_STEP_BYTES: usize = 8 * 1_048_576;

/// How often the indexer reports progress, matching the Apple default.
pub(crate) const INDEX_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

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

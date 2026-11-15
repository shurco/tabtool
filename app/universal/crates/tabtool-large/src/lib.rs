//! Memory-mapped access to CSV files too large to load into memory.
//!
//! [`MappedFile`] maps a file read-only and tracks its identity. [`RowScanner`]
//! finds row boundaries in raw bytes, [`LargeFileIndexer`] scans in the
//! background and publishes a growing [`RowIndex`], and [`LargeCSVSource`]
//! exposes the rows as a `tabtool_core::RowSource`. Edited cells live in
//! [`CellOverlay`]; [`save`] streams the file back with only edited rows
//! rewritten.
//!
//! The crate has no document state and no UI. Threshold routing, undo, save
//! orchestration, the staleness banner and every screen are Plan 3 and Plan 4;
//! `docs/universal.md` records the behavior this layer provides.

#![deny(unsafe_code)]

use std::sync::{Mutex, MutexGuard};

mod block;
mod index;
mod indexer;
mod mapped;
mod overlay;
#[allow(unsafe_code)]
mod platform;
mod saver;
mod scanner;
mod sniffer;
mod source;

pub use block::LargeBlock;
pub use index::RowIndex;
pub use indexer::{IndexCounters, IndexProgress, LargeFileIndexer};
pub use mapped::{FileIdentity, MappedFile, Staleness};
pub use overlay::{CellKey, CellOverlay};
pub use platform::{available_space, replace_file, sync_directory, write_atomic};
pub use saver::{LargeFileSaveError, LargeFileSaver, PreparedSave, save, volume_capacity, write};
pub use scanner::{RowScanner, ScanMode, row_ranges};
pub use sniffer::{check, is_byte_scannable, sniff, validate};
pub use source::{LargeCSVSource, LargeSaveSnapshot};

/// Locks `mutex`, recovering the guard when another thread panicked while
/// holding it.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Limits shared by every large-file component.
pub struct LargeFile;

impl LargeFile {
    /// Files at least this large open in large-file mode.
    pub const THRESHOLD_BYTES: u64 = 100 * 1_048_576;
    /// Rows per index block.
    pub const BLOCK_ROWS: usize = 64;
    /// Bytes sampled when sniffing a format.
    pub const SAMPLE_BYTES: usize = 1_048_576;
    /// Most columns a large file may have.
    pub const MAX_COLUMNS: usize = 2_048;
    /// Most cells copied in one operation.
    pub const MAX_COPY_CELLS: usize = 1_000_000;
}

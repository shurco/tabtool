//! Read-only support for CSV files too large to load into memory.

#![deny(unsafe_code)]

mod mapped;
#[allow(unsafe_code)]
mod platform;

pub use mapped::{FileIdentity, MappedFile, Staleness};
pub use platform::{available_space, replace_file};

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

//! A sparse index of row starts.

use std::ops::Range;

/// A sparse index of row starts: one byte offset per
/// [`crate::LargeFile::BLOCK_ROWS`] rows.
///
/// Block `b` covers the rows `64 * b ..< 64 * (b + 1)`. The index costs
/// 0.125 bytes per row, so a billion rows stay near 125 MB.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RowIndex {
    /// Byte offset of the first row of every block.
    pub checkpoints: Vec<u64>,
    /// Rows that have started in the scanned bytes.
    pub row_count: usize,
    /// The most fields seen in one row, counting separators outside quotes.
    pub max_field_count: usize,
    /// Bytes consumed so far.
    pub scanned_bytes: usize,
    /// Whether the scanned bytes end inside an open quoted field.
    pub ends_inside_quote: bool,
}

impl RowIndex {
    /// Creates an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends the checkpoints of `next`, a scan that continued this one, and
    /// takes over its counters.
    ///
    /// `next` is consumed: its checkpoints move into this index without a copy.
    pub fn extend(&mut self, mut next: RowIndex) {
        self.checkpoints.append(&mut next.checkpoints);
        self.row_count = next.row_count;
        self.max_field_count = next.max_field_count;
        self.scanned_bytes = next.scanned_bytes;
        self.ends_inside_quote = next.ends_inside_quote;
    }

    /// The number of rows in `block`, at most
    /// [`crate::LargeFile::BLOCK_ROWS`].
    pub fn block_row_count(&self, block: usize) -> usize {
        self.row_count
            .saturating_sub(block.saturating_mul(crate::LargeFile::BLOCK_ROWS))
            .min(crate::LargeFile::BLOCK_ROWS)
    }

    /// The byte range of `block`, or `None` when the block is not indexed.
    ///
    /// The last block ends at `file_size`.
    pub fn block_range(&self, block: usize, file_size: usize) -> Option<Range<usize>> {
        let start = usize::try_from(*self.checkpoints.get(block)?).ok()?;
        let end = match self.checkpoints.get(block + 1) {
            Some(&next) => usize::try_from(next).ok()?,
            None => file_size,
        };
        Some(start..start.max(end))
    }
}

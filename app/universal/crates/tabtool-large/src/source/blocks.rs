//! The decoded-block cache behind [`LargeCSVSource`].

use std::sync::Arc;

use super::LargeCSVSource;
use crate::block::LargeBlock;
use crate::index::RowIndex;
use crate::{LargeFile, lock};

const CACHE_CAPACITY: usize = 64;

/// One decoded 64-row block in the cache.
pub(super) struct CachedBlock {
    block: Arc<LargeBlock>,
    /// The index end the block was decoded at; a longer block re-decodes.
    end: usize,
    last_use: u64,
}

impl LargeCSVSource {
    // MARK: Blocks

    /// Rows whose end is known: all of them once indexing is done, else all but
    /// the one still being scanned.
    pub(super) fn visible_rows(&self, scanned_bytes: usize, row_count: usize) -> usize {
        if scanned_bytes >= self.file.size() {
            row_count
        } else {
            row_count.saturating_sub(1)
        }
    }

    pub(super) fn header_offset(&self) -> usize {
        lock(&self.state).header_rows
    }

    pub(super) fn base_cell(&self, physical: usize, column: usize) -> String {
        let Some(block) = self.block(physical / LargeFile::BLOCK_ROWS) else {
            return String::new();
        };
        let row = physical % LargeFile::BLOCK_ROWS;
        block
            .rows
            .get(row)
            .and_then(|fields| fields.get(column))
            .cloned()
            .unwrap_or_default()
    }

    /// The block index, refreshed when the scan has advanced. Only a new scan
    /// position copies the checkpoints; cached blocks then validate their end
    /// against it.
    fn index(&self) -> Arc<RowIndex> {
        let counters = self.indexer.counters();
        {
            let state = lock(&self.state);
            if state.index.scanned_bytes == counters.scanned_bytes
                && state.index.row_count == counters.row_count
            {
                return Arc::clone(&state.index);
            }
        }
        let fresh = Arc::new(self.indexer.snapshot());
        let mut state = lock(&self.state);
        // A thread that read older counters must not replace a newer index.
        if fresh.scanned_bytes > state.index.scanned_bytes
            || (fresh.scanned_bytes == state.index.scanned_bytes
                && fresh.row_count > state.index.row_count)
        {
            state.index = fresh;
        }
        Arc::clone(&state.index)
    }

    /// The decoded block, from the cache or the file; `None` when it is not
    /// indexed or the mapped file was truncated or rewritten. A file replaced
    /// at the path (a save) still reads from the mapping.
    pub(super) fn block(&self, number: usize) -> Option<Arc<LargeBlock>> {
        let index = self.index();
        let file_size = index.scanned_bytes.min(self.file.size());
        let range = index.block_range(number, file_size)?;
        if let Some(hit) = self.lookup(number, range.end) {
            return Some(hit);
        }
        if !self.file.is_readable() {
            return None;
        }
        let bytes = self.file.with_bytes(range.clone(), <[u8]>::to_vec);
        let block = Arc::new(LargeBlock::decode_indexed(
            &bytes,
            &self.configuration,
            index.block_row_count(number),
        ));
        Some(self.store(number, block, range.end))
    }

    fn lookup(&self, number: usize, end: usize) -> Option<Arc<LargeBlock>> {
        let mut state = lock(&self.state);
        state.use_counter += 1;
        let last_use = state.use_counter;
        let is_hit = state
            .cache
            .get(&number)
            .is_some_and(|entry| entry.end == end);
        if !is_hit {
            state.cache_misses += 1;
            return None;
        }
        state.cache_hits += 1;
        let entry = state.cache.get_mut(&number).expect("the entry was checked");
        entry.last_use = last_use;
        Some(Arc::clone(&entry.block))
    }

    fn store(&self, number: usize, block: Arc<LargeBlock>, end: usize) -> Arc<LargeBlock> {
        let mut state = lock(&self.state);
        state.use_counter += 1;
        let last_use = state.use_counter;
        state.cache.insert(
            number,
            CachedBlock {
                block: Arc::clone(&block),
                end,
                last_use,
            },
        );
        if state.cache.len() > CACHE_CAPACITY
            && let Some(oldest) = state
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_use)
                .map(|(&number, _)| number)
        {
            state.cache.remove(&oldest);
        }
        block
    }
}

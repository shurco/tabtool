//! A [`RowSource`] over a memory-mapped file.

mod blocks;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tabtool_core::{
    ColumnId, CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, Encoding, RowSource, issue,
    new_column_id,
};

use blocks::CachedBlock;

use crate::index::RowIndex;
use crate::indexer::LargeFileIndexer;
use crate::mapped::{MappedFile, Staleness};
use crate::overlay::{CellKey, CellOverlay};
use crate::{LargeFile, lock};

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Everything a streaming save needs, captured at one moment.
#[derive(Clone, Debug)]
pub struct LargeSaveSnapshot {
    /// The mapped file the rows came from.
    pub file: Arc<MappedFile>,
    /// The index of the scanned bytes.
    pub index: RowIndex,
    /// The format the file was opened with.
    pub configuration: CsvConfiguration,
    /// Whether the file starts with a UTF-8 byte order mark.
    pub had_bom: bool,
    /// The edited cells, keyed by physical row.
    pub edits: HashMap<CellKey, String>,
}

/// The mutable state behind one lock: the cache, its counters, the column ids
/// and the header row count.
struct SourceState {
    cache: HashMap<usize, CachedBlock>,
    use_counter: u64,
    ids: Vec<ColumnId>,
    header_rows: usize,
    cache_hits: usize,
    cache_misses: usize,
    /// The latest block index, replaced when the scan advances.
    index: Arc<RowIndex>,
}

/// A [`RowSource`] over a memory-mapped file.
///
/// Only the 64-row block holding a requested cell is copied out of the mapping
/// and parsed (with [`crate::LargeBlock`]), then kept in a small LRU cache.
/// Edited cells live in the [`CellOverlay`] and win over the file's bytes. Rows
/// still being indexed are hidden until their end is known.
pub struct LargeCSVSource {
    file: Arc<MappedFile>,
    /// The format the file was opened with. `first_row_as_header` is its value
    /// at creation; [`LargeCSVSource::set_header_enabled`] changes the live
    /// setting.
    pub configuration: CsvConfiguration,
    indexer: Arc<LargeFileIndexer>,
    /// The edited cells, which win over the file's bytes.
    pub overlay: CellOverlay,
    /// Whether the file starts with a UTF-8 byte order mark.
    pub had_utf8_bom: bool,
    state: Mutex<SourceState>,
}

impl LargeCSVSource {
    /// Creates a source over an open file and its indexer.
    ///
    /// `column_ids` carries the column identities of a source this one
    /// replaces (the same file after a save), so the grid keeps its columns.
    pub fn new(
        file: Arc<MappedFile>,
        configuration: CsvConfiguration,
        indexer: Arc<LargeFileIndexer>,
        overlay: CellOverlay,
        column_ids: Vec<ColumnId>,
    ) -> Self {
        let had_utf8_bom = configuration.encoding == Encoding::Utf8
            && file.with_bytes(0..3, |bytes| bytes == UTF8_BOM);
        let header_rows = usize::from(configuration.first_row_as_header);
        let index = Arc::new(indexer.snapshot());
        Self {
            file,
            configuration,
            indexer,
            overlay,
            had_utf8_bom,
            state: Mutex::new(SourceState {
                cache: HashMap::new(),
                use_counter: 0,
                ids: column_ids,
                header_rows,
                cache_hits: 0,
                cache_misses: 0,
                index,
            }),
        }
    }

    // MARK: Indexing

    /// Whether the whole file has been indexed.
    pub fn is_indexed(&self) -> bool {
        self.indexer.is_finished()
    }

    /// The share of the file scanned so far, from 0 to 1; 1 for an empty file.
    pub fn indexing_fraction(&self) -> f64 {
        let size = self.file.size();
        if size == 0 {
            return 1.0;
        }
        (self.indexer.counters().scanned_bytes as f64 / size as f64).min(1.0)
    }

    // MARK: RowSource

    /// The number of rows shown, excluding the header.
    pub fn row_count(&self) -> usize {
        let counters = self.indexer.counters();
        self.visible_rows(counters.scanned_bytes, counters.row_count)
            .saturating_sub(self.header_offset())
    }

    /// The number of columns.
    pub fn column_count(&self) -> usize {
        self.indexer
            .counters()
            .max_field_count
            .min(LargeFile::MAX_COLUMNS)
    }

    /// The identities of the columns, in visual order.
    pub fn column_ids(&self) -> Vec<ColumnId> {
        let count = self.column_count();
        let mut state = lock(&self.state);
        while state.ids.len() < count {
            state.ids.push(new_column_id());
        }
        state.ids[..count].to_vec()
    }

    /// The column names, or `None` when the data has no header row.
    pub fn headers(&self) -> Option<Vec<String>> {
        if self.header_offset() != 1 {
            return None;
        }
        let block = self.block(0)?;
        let row = block.rows.first()?;
        let count = self.column_count();
        Some(
            (0..count)
                .map(|column| row.get(column).cloned().unwrap_or_default())
                .collect(),
        )
    }

    /// Returns the cell text, or an empty string for any invalid index.
    pub fn cell(&self, row: usize, column: usize) -> String {
        if row >= self.row_count() || column >= self.column_count() {
            return String::new();
        }
        let physical = row + self.header_offset();
        if let Some(edited) = self.overlay.get(CellKey {
            row: physical,
            column,
        }) {
            return edited;
        }
        self.base_cell(physical, column)
    }

    /// Treats the first row as the header, or as data again. The file's bytes
    /// and the edits are untouched; only the mapping from rows to file rows
    /// moves.
    pub fn set_header_enabled(&self, enabled: bool) {
        lock(&self.state).header_rows = usize::from(enabled);
    }

    // MARK: Editing

    /// Whether the file on disk still matches the mapped one.
    pub fn staleness(&self) -> Staleness {
        self.file.is_stale()
    }

    /// The file row behind a displayed row.
    pub fn physical_row(&self, display_row: usize) -> usize {
        display_row + self.header_offset()
    }

    /// Whether `row` can be edited: indexing is done, the file is unchanged and
    /// the row's bytes were read faithfully.
    pub fn is_editable(&self, row: usize) -> bool {
        if self.indexer.counters().scanned_bytes < self.file.size()
            || row >= self.row_count()
            || self.file.is_stale() != Staleness::Unchanged
        {
            return false;
        }
        self.block((row + self.header_offset()) / LargeFile::BLOCK_ROWS)
            .is_some_and(|block| block.is_faithful)
    }

    /// Sets a cell and returns the previous override, or `None` when there was
    /// none.
    ///
    /// Setting the file's own text removes the override. Fails with
    /// [`CsvError::RowNotEditable`], [`CsvError::AmbiguousOutput`] or
    /// [`CsvError::UnencodableText`].
    pub fn set_cell(
        &self,
        text: &str,
        row: usize,
        column: usize,
    ) -> Result<Option<String>, CsvError> {
        if column >= self.column_count() || !self.is_editable(row) {
            return Err(CsvError::RowNotEditable);
        }
        self.validate(text)?;
        let physical = row + self.header_offset();
        let key = CellKey {
            row: physical,
            column,
        };
        let previous = self.overlay.get(key);
        if text == self.base_cell(physical, column) {
            self.overlay.remove(key);
        } else {
            self.overlay.set(text.to_string(), key);
        }
        Ok(previous)
    }

    /// The state a streaming save needs: the file, its index and the edits so
    /// far.
    pub fn save_snapshot(&self) -> LargeSaveSnapshot {
        LargeSaveSnapshot {
            file: Arc::clone(&self.file),
            index: self.indexer.snapshot(),
            configuration: self.configuration.clone(),
            had_bom: self.had_utf8_bom,
            edits: self.overlay.copy(),
        }
    }

    /// The cache hits and misses so far.
    pub fn cache_stats(&self) -> (usize, usize) {
        let state = lock(&self.state);
        (state.cache_hits, state.cache_misses)
    }

    fn validate(&self, text: &str) -> Result<(), CsvError> {
        let table = CsvTable::from_rows(vec![vec![text.to_string()]], None, 0);
        match issue(&table, None, &self.configuration) {
            Some(CsvWriteIssue::AmbiguousOutput) => Err(CsvError::AmbiguousOutput),
            Some(CsvWriteIssue::UnencodableText) => Err(CsvError::UnencodableText),
            None => Ok(()),
        }
    }
}

impl RowSource for LargeCSVSource {
    fn row_count(&self) -> usize {
        LargeCSVSource::row_count(self)
    }

    fn column_count(&self) -> usize {
        LargeCSVSource::column_count(self)
    }

    fn column_ids(&self) -> Vec<ColumnId> {
        LargeCSVSource::column_ids(self)
    }

    fn headers(&self) -> Option<Vec<String>> {
        LargeCSVSource::headers(self)
    }

    fn cell(&self, row: usize, column: usize) -> String {
        LargeCSVSource::cell(self, row, column)
    }
}

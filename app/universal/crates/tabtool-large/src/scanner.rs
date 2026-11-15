//! Finds row boundaries in raw bytes without decoding them.

mod block;

use std::ops::Range;

use tabtool_core::CsvConfiguration;

use crate::LargeFile;
use crate::index::RowIndex;

const BLOCK_BYTES: usize = 64;
const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

/// Which scanning path [`RowScanner::scan`] uses.
///
/// Applications use [`ScanMode::Automatic`]. The other modes exist so tests can
/// compare the scalar and block paths.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScanMode {
    /// The block path when the configuration allows it, else the scalar path.
    #[default]
    Automatic,
    /// One byte at a time.
    Scalar,
    /// The block path when the configuration allows it, else the scalar path.
    Block,
}

/// Finds row boundaries in raw bytes without decoding them.
///
/// Rows end at LF, CR or CRLF outside quotes; a final terminator adds no row
/// and an empty input is one row. Scanning is resumable: quote state and a
/// pending CR carry across calls, so a file can be fed in any chunking.
///
/// The scanner looks only at the bytes of the separator, quote and escape
/// characters, so it supports ASCII-compatible encodings and single-byte
/// delimiters. Quote handling is lenient: it follows quote parity (or the
/// dedicated escape) and never fails, so malformed input yields a best-effort
/// index instead of an error.
pub struct RowScanner {
    mode: ScanMode,
    separator: Option<u8>,
    quote: Option<u8>,
    escape: Option<u8>,
    in_quote: bool,
    escaped: bool,
    after_carriage_return: bool,
    at_row_start: bool,
    row_start: usize,
    separators: usize,
}

impl RowScanner {
    /// Prepares a scanner for `configuration`.
    pub fn new(configuration: &CsvConfiguration) -> Self {
        let separator = single_byte(&configuration.column_separator);
        let quote = single_byte(&configuration.quote_character);
        let escape = single_byte(&configuration.escape_character);
        Self {
            mode: ScanMode::Automatic,
            separator,
            quote,
            escape: if escape == quote { None } else { escape },
            in_quote: false,
            escaped: false,
            after_carriage_return: false,
            at_row_start: true,
            row_start: 0,
            separators: 0,
        }
    }

    /// Selects the scanning path. Tests use this to compare the paths.
    #[doc(hidden)]
    pub fn set_mode(&mut self, mode: ScanMode) {
        self.mode = mode;
    }

    /// Scans `bytes`, which start at absolute offset `base`, into `index`.
    ///
    /// The first chunk of a scan starts at offset 0; each later chunk starts
    /// where the previous one ended. Pass `final_chunk: true` with the last
    /// chunk.
    pub fn scan(&mut self, bytes: &[u8], base: usize, index: &mut RowIndex, final_chunk: bool) {
        let mut offset = 0;
        if self.mode != ScanMode::Scalar && self.allows_block_path() {
            while offset + BLOCK_BYTES <= bytes.len() {
                self.scan_block(&bytes[offset..offset + BLOCK_BYTES], base + offset, index);
                offset += BLOCK_BYTES;
            }
        }
        self.scan_bytes(&bytes[offset..], base + offset, index);
        self.finish_chunk(base + bytes.len(), index, final_chunk);
    }

    /// Whether the masks of the block path can reproduce the scalar order.
    ///
    /// The scalar path checks quote, separator and row break in that order. A
    /// byte that is both a separator and a terminator, or both a quote and a
    /// separator, would need two classifications, so those configurations stay
    /// on the scalar path.
    fn allows_block_path(&self) -> bool {
        let quote_clash = matches!(self.quote, Some(LINE_FEED | CARRIAGE_RETURN));
        let separator_clash = matches!(self.separator, Some(LINE_FEED | CARRIAGE_RETURN));
        let same_quote_and_separator = self.quote.is_some() && self.quote == self.separator;
        self.escape.is_none() && !quote_clash && !separator_clash && !same_quote_and_separator
    }

    fn scan_bytes(&mut self, bytes: &[u8], base: usize, index: &mut RowIndex) {
        for (position, &byte) in bytes.iter().enumerate() {
            if self.after_carriage_return {
                self.after_carriage_return = false;
                if byte == LINE_FEED {
                    self.row_start = base + position + 1;
                    continue;
                }
            }
            if self.at_row_start {
                self.begin_row(index);
            }
            if self.in_quote {
                if self.escaped {
                    self.escaped = false;
                } else if Some(byte) == self.escape {
                    self.escaped = true;
                } else if Some(byte) == self.quote {
                    self.in_quote = false;
                }
            } else if Some(byte) == self.quote {
                self.in_quote = true;
            } else if Some(byte) == self.separator {
                self.separators += 1;
            } else if byte == LINE_FEED {
                self.end_row(base + position + 1, index);
            } else if byte == CARRIAGE_RETURN {
                self.end_row(base + position + 1, index);
                self.after_carriage_return = true;
            }
        }
    }

    fn finish_chunk(&mut self, end: usize, index: &mut RowIndex, final_chunk: bool) {
        if final_chunk && index.row_count == 0 && self.at_row_start {
            self.begin_row(index);
        }
        if !self.at_row_start {
            index.max_field_count = index.max_field_count.max(self.separators + 1);
        }
        index.scanned_bytes = end;
        index.ends_inside_quote = self.in_quote;
    }

    fn begin_row(&mut self, index: &mut RowIndex) {
        if index.row_count.is_multiple_of(LargeFile::BLOCK_ROWS) {
            index.checkpoints.push(self.row_start as u64);
        }
        index.row_count += 1;
        self.at_row_start = false;
        self.separators = 0;
    }

    fn end_row(&mut self, next_start: usize, index: &mut RowIndex) {
        index.max_field_count = index.max_field_count.max(self.separators + 1);
        self.row_start = next_start;
        self.at_row_start = true;
    }
}

fn single_byte(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    if bytes.len() == 1 {
        Some(bytes[0])
    } else {
        None
    }
}

/// The byte range of every row in `bytes`, terminator included.
///
/// `bytes` must start at a row start outside any quote, as an index block does.
/// The ranges follow the same row rules as [`RowScanner::scan`]: LF, CRLF and
/// CR each end one row, and the range runs to the next row's start, or to the
/// end of `bytes` for the last row. Empty input is one empty row.
pub fn row_ranges(bytes: &[u8], configuration: &CsvConfiguration) -> Vec<Range<usize>> {
    let mut scanner = RowScanner::new(configuration);
    let mut index = RowIndex::new();
    let mut starts = Vec::new();
    for position in 0..bytes.len() {
        let rows_before = index.row_count;
        let start_before = scanner.row_start;
        scanner.scan_bytes(&bytes[position..=position], position, &mut index);
        if index.row_count > rows_before {
            starts.push(start_before);
        }
    }
    // Empty input is one row with an empty range.
    if starts.is_empty() {
        starts.push(0);
    }
    starts
        .iter()
        .enumerate()
        .map(|(row, &start)| {
            let end = starts.get(row + 1).copied().unwrap_or(bytes.len());
            start..end
        })
        .collect()
}

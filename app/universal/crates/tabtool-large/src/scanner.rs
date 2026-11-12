//! Finds row boundaries in raw bytes without decoding them.

use std::ops::Range;

use tabtool_core::CsvConfiguration;

use crate::LargeFile;
use crate::index::RowIndex;

const BLOCK_BYTES: usize = 64;
const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const BYTE_LOW_BITS: u64 = 0x7f7f_7f7f_7f7f_7f7f;
const BYTE_HIGH_BITS: u64 = 0x8080_8080_8080_8080;
const GATHER_HIGH_BITS: u64 = 0x0002_0408_1020_4081;

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
        if self.mode != ScanMode::Scalar && self.escape.is_none() {
            while offset + BLOCK_BYTES <= bytes.len() {
                self.scan_block(&bytes[offset..offset + BLOCK_BYTES], base + offset, index);
                offset += BLOCK_BYTES;
            }
        }
        self.scan_bytes(&bytes[offset..], base + offset, index);
        self.finish_chunk(base + bytes.len(), index, final_chunk);
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

    /// Scans one full 64-byte block starting at absolute offset `base`.
    ///
    /// The block path classifies 64 bytes at a time into bit masks (one bit per
    /// byte) and derives row ends from them, so quote state costs a prefix-xor
    /// instead of a branch per byte. It requires that a quote is escaped by
    /// doubling (or quoting is off), because a dedicated escape character would
    /// make the quote state depend on the previous byte.
    fn scan_block(&mut self, block: &[u8], base: usize, index: &mut RowIndex) {
        let (quote_mask, line_feeds, carriage_returns, separator_mask) = self.block_masks(block);

        let mut parity = quote_mask;
        parity ^= parity << 1;
        parity ^= parity << 2;
        parity ^= parity << 4;
        parity ^= parity << 8;
        parity ^= parity << 16;
        parity ^= parity << 32;
        let inside = parity ^ if self.in_quote { u64::MAX } else { 0 };

        let outside_line_feeds = line_feeds & !inside;
        let outside_carriage_returns = carriage_returns & !inside;
        let carry = if self.after_carriage_return { 1 } else { 0 };
        let swallowed = outside_line_feeds & ((outside_carriage_returns << 1) | carry);
        let separators_outside = separator_mask & !inside;

        let mut row_ends = (outside_line_feeds | outside_carriage_returns) & !swallowed;
        let mut segment_start = 0;
        while segment_start < BLOCK_BYTES {
            let terminated = row_ends != 0;
            let last = if terminated {
                row_ends.trailing_zeros() as usize
            } else {
                BLOCK_BYTES - 1
            };
            row_ends &= row_ends.wrapping_sub(1);

            let mut first = segment_start;
            if self.at_row_start {
                if (swallowed >> first) & 1 == 1 {
                    self.row_start = base + first + 1;
                    first += 1;
                }
                if first <= last {
                    self.begin_row(index);
                }
            }
            if first <= last {
                self.separators +=
                    (separators_outside & bit_range(first, last)).count_ones() as usize;
            }
            if terminated {
                self.end_row(base + last + 1, index);
            }
            segment_start = last + 1;
        }

        self.after_carriage_return = (outside_carriage_returns >> 63) & 1 == 1;
        self.in_quote = (inside >> 63) & 1 == 1;
    }

    /// The quote, line-feed, carriage-return and separator masks of `block`.
    fn block_masks(&self, block: &[u8]) -> (u64, u64, u64, u64) {
        let mut quote_mask = 0;
        let mut line_feeds = 0;
        let mut carriage_returns = 0;
        let mut separator_mask = 0;
        for (word_index, word) in block.as_chunks::<8>().0.iter().enumerate() {
            let word = u64::from_le_bytes(*word);
            let shift = word_index * 8;
            if let Some(quote) = self.quote {
                quote_mask |= byte_mask(word, quote) << shift;
            }
            line_feeds |= byte_mask(word, LINE_FEED) << shift;
            carriage_returns |= byte_mask(word, CARRIAGE_RETURN) << shift;
            if let Some(separator) = self.separator {
                separator_mask |= byte_mask(word, separator) << shift;
            }
        }
        (quote_mask, line_feeds, carriage_returns, separator_mask)
    }
}

/// One bit per byte of `word`, set where the byte equals `target`.
///
/// The zero-byte test uses the exact lane arithmetic `((x & 0x7f) + 0x7f) | x`,
/// whose carry cannot cross a lane, and the multiplication gathers the eight
/// lane bits into the low byte without collisions.
#[inline]
fn byte_mask(word: u64, target: u8) -> u64 {
    let matches = word ^ u64::from(target).wrapping_mul(0x0101_0101_0101_0101);
    let zero_bytes =
        (!(((matches & BYTE_LOW_BITS).wrapping_add(BYTE_LOW_BITS)) | matches)) & BYTE_HIGH_BITS;
    zero_bytes.wrapping_mul(GATHER_HIGH_BITS) >> 56
}

/// Bits `low..=high` set, for bit indices `0..=63`.
fn bit_range(low: usize, high: usize) -> u64 {
    let upper = if high >= 63 {
        u64::MAX
    } else {
        (1 << (high + 1)) - 1
    };
    let lower = (1 << low) - 1;
    upper & !lower
}

fn single_byte(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    if bytes.len() == 1 {
        Some(bytes[0])
    } else {
        None
    }
}

/// The byte range of every row in `bytes`, terminator excluded.
///
/// `bytes` must start at a row start outside any quote, as an index block does.
/// The ranges follow the same row rules as [`RowScanner::scan`]: LF, CRLF and
/// CR each end one row and are not part of its range. Empty input is one empty
/// row. The saver reads each row's terminator from the bytes after its range.
pub fn row_ranges(bytes: &[u8], configuration: &CsvConfiguration) -> Vec<Range<usize>> {
    let mut scanner = RowScanner::new(configuration);
    let mut index = RowIndex::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    for (position, &byte) in bytes.iter().enumerate() {
        let rows_before = index.row_count;
        let start_before = scanner.row_start;
        let was_after_carriage_return = scanner.after_carriage_return;
        scanner.scan_bytes(&bytes[position..=position], position, &mut index);
        if index.row_count > rows_before {
            starts.push(start_before);
        }
        let swallowed = was_after_carriage_return && byte == LINE_FEED;
        if !swallowed && scanner.at_row_start && (byte == LINE_FEED || byte == CARRIAGE_RETURN) {
            ends.push(position);
        }
    }
    // Empty input is one row with an empty range.
    if starts.is_empty() {
        starts.push(0);
    }
    starts
        .iter()
        .enumerate()
        .map(|(row, &start)| start..ends.get(row).copied().unwrap_or(bytes.len()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{bit_range, byte_mask};

    /// A local SplitMix64 stream so the mask test needs no dependency.
    struct SplitMix64Probe {
        state: u64,
    }

    impl SplitMix64Probe {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next(&mut self) -> u64 {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = self.state;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            value ^ (value >> 31)
        }
    }

    #[test]
    fn byte_mask_matches_naive_classification() {
        let mut generator = SplitMix64Probe::new(0x5EED_1234);
        let targets = [0u8, 1, 9, 10, 13, 34, 44, 127, 128, 255];
        for _ in 0..10_000 {
            let word = generator.next();
            for target in targets {
                let mut expected = 0;
                for lane in 0..8 {
                    if ((word >> (8 * lane)) & 0xff) as u8 == target {
                        expected |= 1 << lane;
                    }
                }
                assert_eq!(byte_mask(word, target), expected, "word {word:#x}");
            }
        }
    }

    #[test]
    fn bit_range_covers_inclusive_bounds() {
        assert_eq!(bit_range(0, 0), 1);
        assert_eq!(bit_range(0, 63), u64::MAX);
        assert_eq!(bit_range(63, 63), 1 << 63);
        assert_eq!(bit_range(3, 5), 0b111 << 3);
    }
}

//! The 64-byte block path of [`RowScanner`].

use super::{BLOCK_BYTES, CARRIAGE_RETURN, LINE_FEED, RowScanner};
use crate::index::RowIndex;

const BYTE_LOW_BITS: u64 = 0x7f7f_7f7f_7f7f_7f7f;
const BYTE_HIGH_BITS: u64 = 0x8080_8080_8080_8080;
const GATHER_HIGH_BITS: u64 = 0x0002_0408_1020_4081;

impl RowScanner {
    /// Scans one full 64-byte block starting at absolute offset `base`.
    ///
    /// The block path classifies 64 bytes at a time into bit masks (one bit per
    /// byte) and derives row ends from them, so quote state costs a prefix-xor
    /// instead of a branch per byte. It requires that a quote is escaped by
    /// doubling (or quoting is off), because a dedicated escape character would
    /// make the quote state depend on the previous byte. It is also disabled
    /// when the quote or separator collides with a row terminator or with each
    /// other; see [`RowScanner::allows_block_path`].
    pub(super) fn scan_block(&mut self, block: &[u8], base: usize, index: &mut RowIndex) {
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

#[cfg(test)]
mod tests {
    use tabtool_testdata::SplitMix64;

    use super::{bit_range, byte_mask};

    #[test]
    fn byte_mask_matches_naive_classification() {
        let mut generator = SplitMix64::new(0x5EED_1234);
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

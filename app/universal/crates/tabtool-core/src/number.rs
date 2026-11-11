//! Numeric classification and decimal-mark localization for CSV cell text.
//!
//! ASCII text with a single-byte ASCII decimal mark is scanned byte by byte;
//! anything else falls back to a cached, Unicode-aware regular expression.
//! This mirrors the Apple `CSVNumber` helpers.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use regex::Regex;

/// Returns `true` when `text` is a number using `decimal_mark`.
///
/// The pattern matches an optional sign, digits around the configured decimal
/// mark, an optional exponent, and surrounding whitespace.
pub fn is_numeric(text: &str, decimal_mark: &str) -> bool {
    scan_numeric(text, decimal_mark).unwrap_or_else(|| matches_regex(text, decimal_mark))
}

/// Replaces the decimal mark with `decimal_separator` when `text` is numeric.
///
/// Non-numeric text, and text with no separator to replace it with, is
/// returned unchanged.
pub fn localized(text: &str, decimal_mark: &str, decimal_separator: &str) -> String {
    if decimal_separator.is_empty() || !is_numeric(text, decimal_mark) {
        return text.to_owned();
    }
    text.replace(decimal_mark, decimal_separator)
}

/// Hand-written scan of the numeric pattern for ASCII text and a simple ASCII
/// decimal mark.
///
/// Returns `None` when the input needs the regex: non-ASCII text, or a mark
/// that is not one ASCII byte, is a digit or whitespace, or is one of `eE+-`.
fn scan_numeric(text: &str, decimal_mark: &str) -> Option<bool> {
    let mark = match decimal_mark.as_bytes() {
        [mark] if is_scannable_mark(*mark) => *mark,
        _ => return None,
    };
    let bytes = text.as_bytes();
    if bytes.iter().any(|byte| !byte.is_ascii()) {
        return None;
    }

    let mut index = 0;
    skip_while(bytes, &mut index, is_space);
    if !consume(bytes, &mut index, b'+') {
        let _ = consume(bytes, &mut index, b'-');
    }
    let before = skip_while(bytes, &mut index, is_ascii_digit);
    let mut after = 0;
    if consume(bytes, &mut index, mark) {
        after = skip_while(bytes, &mut index, is_ascii_digit);
    }
    if before + after == 0 {
        return Some(false);
    }
    if consume(bytes, &mut index, b'e') || consume(bytes, &mut index, b'E') {
        if !consume(bytes, &mut index, b'+') {
            let _ = consume(bytes, &mut index, b'-');
        }
        if skip_while(bytes, &mut index, is_ascii_digit) == 0 {
            return Some(false);
        }
    }
    skip_while(bytes, &mut index, is_space);
    Some(index == bytes.len())
}

/// Whether the scanner can handle `mark` itself instead of using the regex.
fn is_scannable_mark(mark: u8) -> bool {
    mark.is_ascii()
        && !mark.is_ascii_digit()
        && !is_space(mark)
        && !matches!(mark, b'e' | b'E' | b'+' | b'-')
}

/// The whitespace the scanner accepts: space and `0x09..=0x0D`.
fn is_space(byte: u8) -> bool {
    byte == 0x20 || (0x09..=0x0D).contains(&byte)
}

fn is_ascii_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

/// Advances `index` past the bytes that satisfy `predicate`, returning the count.
fn skip_while(bytes: &[u8], index: &mut usize, predicate: fn(u8) -> bool) -> usize {
    let start = *index;
    while bytes.get(*index).is_some_and(|byte| predicate(*byte)) {
        *index += 1;
    }
    *index - start
}

/// Consumes `byte` at `index` when it matches.
fn consume(bytes: &[u8], index: &mut usize, byte: u8) -> bool {
    if bytes.get(*index) == Some(&byte) {
        *index += 1;
        true
    } else {
        false
    }
}

fn matches_regex(text: &str, decimal_mark: &str) -> bool {
    numeric_regex(decimal_mark).is_some_and(|regex| regex.is_match(text))
}

/// Compiled patterns per decimal mark. [`Regex`] cloning is cheap; the cache
/// itself is only held while looking a pattern up or compiling a new one.
fn numeric_regex(decimal_mark: &str) -> Option<Regex> {
    static CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();

    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(regex) = cache.get(decimal_mark) {
        return Some(regex.clone());
    }

    let escaped = regex::escape(decimal_mark);
    let pattern = format!(r"^\s*[+-]?(\d+{escaped}?\d*|\d*{escaped}?\d+)([eE][+-]?\d+)?\s*$");
    let regex = Regex::new(&pattern).ok()?;
    cache.insert(decimal_mark.to_owned(), regex.clone());
    Some(regex)
}

#[cfg(test)]
mod tests {
    use super::scan_numeric;

    #[test]
    fn scanner_defers_on_non_ascii_text() {
        assert_eq!(scan_numeric("1\u{00A0}", "."), None);
        assert_eq!(scan_numeric("\u{0663}", "."), None);
        assert_eq!(scan_numeric("\u{0661}\u{0662}\u{0663}", "."), None);
    }

    #[test]
    fn scanner_defers_on_unusual_marks() {
        for mark in ["", "ab", "\u{0663}", "5", "e", "E", "+", "-", " ", "\t"] {
            assert_eq!(scan_numeric("1.5", mark), None, "{mark:?}");
        }
    }

    #[test]
    fn scanner_decides_ascii_input_without_the_regex() {
        assert_eq!(scan_numeric(" 1.5 ", "."), Some(true));
        assert_eq!(scan_numeric("1 2", "."), Some(false));
        assert_eq!(scan_numeric("", "."), Some(false));
    }
}

//! Conversion of the decimal mark in numeric text.

use crate::is_numeric;

/// Returns `text` with `from` replaced by `to` when `text` is numeric using
/// `from`; otherwise returns `text` unchanged.
///
/// An empty source or target mark and identical marks are no-ops.
pub fn convert_decimal_mark(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() || to.is_empty() || from == to || !is_numeric(text, from) {
        return text.to_owned();
    }
    text.replace(from, to)
}

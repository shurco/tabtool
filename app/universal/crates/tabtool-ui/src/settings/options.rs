//! Picker indexes and labels that fall back to the offered defaults.

use tabtool_core::Encoding;
use tabtool_i18n::{Language, tr};
use tabtool_settings::LARGE_FILE_THRESHOLDS_MIB;

use crate::format_options::{DECIMALS, SEPARATORS};

/// The offered index of UTF-8.
pub(super) fn default_encoding_index() -> usize {
    Encoding::ALL
        .iter()
        .position(|encoding| *encoding == Encoding::Utf8)
        .unwrap_or(0)
}

/// The offered index of `separator`, or comma for an unknown value.
pub(super) fn separator_index(separator: &str) -> usize {
    SEPARATORS
        .iter()
        .position(|(candidate, _)| *candidate == separator)
        .unwrap_or(0)
}

/// The offered index of `decimal`, or the dot for an unknown value.
pub(super) fn decimal_index(decimal: &str) -> usize {
    DECIMALS
        .iter()
        .position(|(candidate, _)| *candidate == decimal)
        .unwrap_or(0)
}

/// The offered index of the 100 MiB default threshold.
pub(super) fn default_threshold_index() -> usize {
    LARGE_FILE_THRESHOLDS_MIB
        .iter()
        .position(|mebibytes| *mebibytes == 100)
        .unwrap_or(0)
}

/// The picker label for a large-file threshold: binary units, as the system
/// byte-count formatter shows.
pub(super) fn threshold_label(language: Language, mebibytes: u32) -> String {
    let (key, value) = if mebibytes >= 1024 {
        ("%lld GiB", mebibytes / 1024)
    } else {
        ("%lld MiB", mebibytes)
    };
    tr(language, key).replace("%lld", &value.to_string())
}

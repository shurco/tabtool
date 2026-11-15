//! The settings pickers shared by the desktop tabs and the mobile sheet, with
//! indexes and labels that fall back to the offered defaults.

use tabtool_core::Encoding;
use tabtool_i18n::{Language, tr};
use tabtool_settings::{LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, Settings};

use crate::format_options::{DECIMALS, SEPARATORS};

/// One settings picker: its labels, the selected index and the field it
/// writes. Only the widget that shows it differs between platforms.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SettingPicker {
    Language,
    Encoding,
    Separator,
    Quote,
    DecimalMark,
    /// The row heights the platform offers; the middle one is the default.
    RowHeight(&'static [f32]),
    LargeFileThreshold,
}

impl SettingPicker {
    /// The catalog key of the picker's title.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Language => "Language",
            Self::Encoding => "Encoding",
            Self::Separator => "Separator",
            Self::Quote => "Quote",
            Self::DecimalMark => "Decimal Mark",
            Self::RowHeight(_) => "Row Height",
            Self::LargeFileThreshold => "Large File Threshold",
        }
    }

    /// The translated option labels, in offered order.
    pub(crate) fn labels(self, language: Language) -> Vec<String> {
        let keys: Vec<&str> = match self {
            Self::Language => {
                return Language::ALL
                    .iter()
                    .map(|candidate| {
                        candidate
                            .native_name()
                            .unwrap_or_else(|| tr(language, "System"))
                            .to_owned()
                    })
                    .collect();
            }
            Self::LargeFileThreshold => {
                return LARGE_FILE_THRESHOLDS_MIB
                    .iter()
                    .map(|mebibytes| threshold_label(language, *mebibytes))
                    .collect();
            }
            Self::Encoding => Encoding::ALL
                .iter()
                .map(|encoding| encoding.display_name())
                .collect(),
            Self::Separator => SEPARATORS.iter().map(|(_, key)| *key).collect(),
            Self::Quote => vec!["Doubled", "Backslash", "None"],
            Self::DecimalMark => DECIMALS.iter().map(|(_, key)| *key).collect(),
            Self::RowHeight(_) => vec!["Compact", "Default", "Comfortable"],
        };
        keys.into_iter()
            .map(|key| tr(language, key).to_owned())
            .collect()
    }

    /// The index of the current value, or the offered default.
    pub(crate) fn selected(self, settings: &Settings) -> usize {
        match self {
            Self::Language => Language::ALL
                .iter()
                .position(|candidate| *candidate == settings.app_language)
                .unwrap_or(0),
            Self::Encoding => Encoding::ALL
                .iter()
                .position(|encoding| *encoding == settings.new_document_encoding)
                .unwrap_or_else(default_encoding_index),
            Self::Separator => separator_index(&settings.new_document_separator),
            Self::Quote => settings.new_document_quote_style.index(),
            Self::DecimalMark => decimal_index(&settings.new_document_decimal_mark),
            Self::RowHeight(heights) => heights
                .iter()
                .position(|height| *height == settings.grid_row_height)
                .unwrap_or(1),
            Self::LargeFileThreshold => LARGE_FILE_THRESHOLDS_MIB
                .iter()
                .position(|mebibytes| *mebibytes == settings.large_file_threshold_mib)
                .unwrap_or_else(default_threshold_index),
        }
    }

    /// Writes the option at `index` into `settings`.
    pub(crate) fn apply(self, settings: &mut Settings, index: usize) {
        match self {
            Self::Language => settings.app_language = Language::ALL[index],
            Self::Encoding => settings.new_document_encoding = Encoding::ALL[index],
            Self::Separator => settings.new_document_separator = SEPARATORS[index].0.to_owned(),
            Self::Quote => settings.new_document_quote_style = QuoteStyle::ALL[index],
            Self::DecimalMark => {
                settings.new_document_decimal_mark = DECIMALS[index].0.to_owned();
            }
            Self::RowHeight(heights) => settings.grid_row_height = heights[index],
            Self::LargeFileThreshold => {
                settings.large_file_threshold_mib = LARGE_FILE_THRESHOLDS_MIB[index];
            }
        }
    }
}

/// The offered index of UTF-8.
fn default_encoding_index() -> usize {
    Encoding::ALL
        .iter()
        .position(|encoding| *encoding == Encoding::Utf8)
        .unwrap_or(0)
}

/// The offered index of `separator`, or comma for an unknown value.
fn separator_index(separator: &str) -> usize {
    SEPARATORS
        .iter()
        .position(|(candidate, _)| *candidate == separator)
        .unwrap_or(0)
}

/// The offered index of `decimal`, or the dot for an unknown value.
fn decimal_index(decimal: &str) -> usize {
    DECIMALS
        .iter()
        .position(|(candidate, _)| *candidate == decimal)
        .unwrap_or(0)
}

/// The offered index of the 100 MiB default threshold.
fn default_threshold_index() -> usize {
    LARGE_FILE_THRESHOLDS_MIB
        .iter()
        .position(|mebibytes| *mebibytes == 100)
        .unwrap_or(0)
}

/// The picker label for a large-file threshold: binary units, as the system
/// byte-count formatter shows.
fn threshold_label(language: Language, mebibytes: u32) -> String {
    let (key, value) = if mebibytes >= 1024 {
        ("%lld GiB", mebibytes / 1024)
    } else {
        ("%lld MiB", mebibytes)
    };
    tr(language, key).replace("%lld", &value.to_string())
}

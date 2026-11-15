//! The platform-neutral settings model: keys, defaults and derived formats.
//!
//! The model has no file system, dialog or OS dependency. The desktop
//! `tabtool-platform` crate owns the JSON file and the settings location; the
//! UI and the app read the same types from here.

#![forbid(unsafe_code)]

mod quote_style;

use std::path::PathBuf;

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_i18n::{Language, language_from_system};

pub use quote_style::QuoteStyle;

/// The persisted key strings, unchanged from the Apple app's `Preferences.Key`.
pub mod keys {
    /// Whether the format bar is shown.
    pub const SHOW_FORMAT_BAR: &str = "showFormatBar";
    /// The interface language.
    pub const APP_LANGUAGE: &str = "appLanguage";
    /// Encoding of a new document.
    pub const NEW_DOCUMENT_ENCODING: &str = "newDocumentEncoding";
    /// Separator of a new document.
    pub const NEW_DOCUMENT_SEPARATOR: &str = "newDocumentSeparator";
    /// Quote style of a new document.
    pub const NEW_DOCUMENT_QUOTE_STYLE: &str = "newDocumentQuoteStyle";
    /// Decimal mark of a new document.
    pub const NEW_DOCUMENT_DECIMAL_MARK: &str = "newDocumentDecimalMark";
    /// Whether a new document has a header row.
    pub const NEW_DOCUMENT_FIRST_ROW_AS_HEADER: &str = "newDocumentFirstRowAsHeader";
    /// Whether grid rows alternate their background.
    pub const GRID_ALTERNATING_ROWS: &str = "gridAlternatingRows";
    /// The grid row height.
    pub const GRID_ROW_HEIGHT: &str = "gridRowHeight";
    /// Whether the grid shows row numbers.
    pub const GRID_SHOWS_ROW_NUMBERS: &str = "gridShowsRowNumbers";
    /// The large file threshold in MiB.
    pub const LARGE_FILE_THRESHOLD_MIB: &str = "largeFileThresholdMiB";
    /// Review prompt bookkeeping.
    pub const REVIEW_FIRST_LAUNCH: &str = "reviewFirstLaunch";
    /// Review prompt bookkeeping.
    pub const REVIEW_ACTIVE_DAYS: &str = "reviewActiveDays";
    /// Review prompt bookkeeping.
    pub const REVIEW_LAST_ACTIVE_DAY: &str = "reviewLastActiveDay";
    /// Review prompt bookkeeping.
    pub const REVIEW_ENGAGEMENTS: &str = "reviewEngagements";
    /// Review prompt bookkeeping.
    pub const REVIEW_LAST_PROMPTED_VERSION: &str = "reviewLastPromptedVersion";
    /// Review prompt bookkeeping.
    pub const REVIEW_LAST_PROMPTED_AT: &str = "reviewLastPromptedAt";
    /// The most recently opened documents, newest first.
    pub const RECENT_DOCUMENTS: &str = "recentDocuments";
}

/// The row heights, in points, the desktop settings offer.
pub const ROW_HEIGHTS: [f32; 3] = [20.0, 24.0, 32.0];

/// The row heights, in points, the mobile settings offer.
pub const MOBILE_ROW_HEIGHTS: [f32; 3] = [32.0, 44.0, 56.0];

/// Every row height either platform persists.
///
/// The platforms keep separate settings files, but the loader accepts both
/// sets so a value written by one platform stays valid instead of silently
/// reverting to a default.
pub const VALID_ROW_HEIGHTS: [f32; 5] = [20.0, 24.0, 32.0, 44.0, 56.0];

/// The large file thresholds, in MiB, the settings offer.
pub const LARGE_FILE_THRESHOLDS_MIB: [u32; 6] = [25, 50, 100, 250, 500, 1024];

/// How many recent documents the settings keep.
pub const RECENT_DOCUMENTS_LIMIT: usize = 10;

/// The desktop row height used when the stored value is not one the settings
/// offer.
pub const DEFAULT_ROW_HEIGHT: f32 = 24.0;

/// The mobile row height used when the stored value is not one the mobile
/// settings offer.
pub const MOBILE_DEFAULT_ROW_HEIGHT: f32 = 44.0;

/// Everything the app persists between launches.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Whether the format bar is shown.
    pub show_format_bar: bool,
    /// The interface language.
    pub app_language: Language,
    /// Encoding of a new document.
    pub new_document_encoding: Encoding,
    /// Separator of a new document.
    pub new_document_separator: String,
    /// Quote style of a new document.
    pub new_document_quote_style: QuoteStyle,
    /// Decimal mark of a new document.
    pub new_document_decimal_mark: String,
    /// Whether a new document has a header row.
    pub new_document_first_row_as_header: bool,
    /// Whether grid rows alternate their background.
    pub grid_alternating_rows: bool,
    /// The grid row height.
    pub grid_row_height: f32,
    /// Whether the grid shows row numbers.
    pub grid_shows_row_numbers: bool,
    /// The large file threshold in MiB.
    pub large_file_threshold_mib: u32,
    /// The most recently opened documents, newest first.
    pub recent_documents: Vec<PathBuf>,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_first_launch: Option<String>,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_active_days: u32,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_last_active_day: Option<String>,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_engagements: u32,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_last_prompted_version: Option<String>,
    /// Review prompt bookkeeping, persisted unchanged.
    pub review_last_prompted_at: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_format_bar: true,
            app_language: Language::System,
            new_document_encoding: Encoding::Utf8,
            new_document_separator: ",".to_string(),
            new_document_quote_style: QuoteStyle::Doubled,
            new_document_decimal_mark: ".".to_string(),
            new_document_first_row_as_header: false,
            grid_alternating_rows: false,
            grid_row_height: DEFAULT_ROW_HEIGHT,
            grid_shows_row_numbers: true,
            large_file_threshold_mib: 100,
            recent_documents: Vec::new(),
            review_first_launch: None,
            review_active_days: 0,
            review_last_active_day: None,
            review_engagements: 0,
            review_last_prompted_version: None,
            review_last_prompted_at: None,
        }
    }
}

impl Settings {
    /// The interface language for this preference and the system languages.
    ///
    /// [`Language::System`] follows the first system language through the
    /// catalog mapping; an explicit preference always wins.
    pub fn interface_language(&self, system_tags: &[String]) -> Language {
        match self.app_language {
            Language::System => language_from_system(system_tags),
            language => language,
        }
    }

    /// The format a new document starts with.
    pub fn new_document_configuration(&self) -> CsvConfiguration {
        CsvConfiguration {
            encoding: self.new_document_encoding,
            column_separator: self.new_document_separator.clone(),
            quote_character: self.new_document_quote_style.quote_character().to_string(),
            escape_character: self.new_document_quote_style.escape_character().to_string(),
            decimal_mark: self.new_document_decimal_mark.clone(),
            first_row_as_header: self.new_document_first_row_as_header,
        }
    }

    /// The file size from which a document opens in large file mode.
    pub fn large_file_threshold_bytes(&self) -> u64 {
        u64::from(self.large_file_threshold_mib) * 1_048_576
    }

    /// The grid row height for the desktop layout, or the desktop default
    /// when the stored value is not one the desktop settings offer.
    pub fn row_height(&self) -> f32 {
        if ROW_HEIGHTS.contains(&self.grid_row_height) {
            self.grid_row_height
        } else {
            DEFAULT_ROW_HEIGHT
        }
    }
}

/// Returns `value` when either platform offers it, otherwise the desktop
/// default.
pub fn valid_row_height(value: f32) -> f32 {
    if VALID_ROW_HEIGHTS.contains(&value) {
        value
    } else {
        DEFAULT_ROW_HEIGHT
    }
}

/// Returns `value` when the mobile settings offer it, otherwise the mobile
/// default.
pub fn valid_mobile_row_height(value: f32) -> f32 {
    if MOBILE_ROW_HEIGHTS.contains(&value) {
        value
    } else {
        MOBILE_DEFAULT_ROW_HEIGHT
    }
}

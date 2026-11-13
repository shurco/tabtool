//! The settings the app persists as JSON.

mod encoding;
mod persisted;
mod quote_style;

use std::path::{Path, PathBuf};

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_i18n::Language;

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

/// The row heights, in points, the settings offer.
pub const ROW_HEIGHTS: [f32; 3] = [20.0, 24.0, 32.0];

/// The large file thresholds, in MiB, the settings offer.
pub const LARGE_FILE_THRESHOLDS_MIB: [u32; 6] = [25, 50, 100, 250, 500, 1024];

/// How many recent documents the settings keep.
pub const RECENT_DOCUMENTS_LIMIT: usize = 10;

/// The row height used when the stored value is not one the settings offer.
const DEFAULT_ROW_HEIGHT: f32 = 24.0;

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
    /// Loads the settings at `path`.
    ///
    /// A missing or malformed file and every missing, mistyped or unknown
    /// value fall back to the default, so a stale file can never reach the
    /// document or the grid.
    pub fn load(path: &Path) -> Self {
        persisted::load(path)
    }

    /// Atomically writes the settings to `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        persisted::save(self, path)
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

    /// The grid row height, or the default when the stored value is not offered.
    pub fn row_height(&self) -> f32 {
        valid_row_height(self.grid_row_height)
    }
}

pub(crate) fn valid_row_height(value: f32) -> f32 {
    if ROW_HEIGHTS.contains(&value) {
        value
    } else {
        DEFAULT_ROW_HEIGHT
    }
}

/// The settings file in the platform config directory.
///
/// The identifier matches the bundle id `app.shurco.tabtool`: on macOS that
/// yields `~/Library/Application Support/app.shurco.tabtool/settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("app", "shurco", "tabtool")
        .map(|directories| directories.config_dir().join("settings.json"))
}

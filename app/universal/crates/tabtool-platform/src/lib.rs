//! Desktop services, persisted settings and system locale for TabTool.

#![forbid(unsafe_code)]

pub mod desktop;
pub mod locale;
pub mod settings;

pub use desktop::{Clipboard, DesktopPlatform, FileDialogs, OpenUrl, TABLE_EXTENSIONS};
pub use locale::{SystemInfo, SystemLocale, decimal_separator, system_locale};
pub use settings::{
    DEFAULT_ROW_HEIGHT, LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, RECENT_DOCUMENTS_LIMIT, ROW_HEIGHTS,
    Settings, keys, load_settings, save_settings, settings_path, valid_row_height,
};

//! Platform services, settings and system locale for TabTool.

#![deny(unsafe_code)]

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
pub mod android;
#[cfg(not(target_os = "android"))]
pub mod desktop;
pub mod locale;
pub mod services;
pub mod settings;
pub mod text_input;

#[cfg(not(target_os = "android"))]
pub use desktop::{DesktopPlatform, FileDialogs};
pub use locale::{SystemInfo, decimal_separator};
#[cfg(not(target_os = "android"))]
pub use locale::{SystemLocale, system_locale};
pub use services::{
    BackHandler, Clipboard, DocumentFiles, Insets, OpenUrl, Share, TABLE_EXTENSIONS,
};
pub use settings::{
    DEFAULT_ROW_HEIGHT, LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, RECENT_DOCUMENTS_LIMIT, ROW_HEIGHTS,
    Settings, keys, load_settings, save_settings, settings_path, valid_row_height,
};
pub use text_input::{TextInput, TextInputOutcome, TextInputRequest};

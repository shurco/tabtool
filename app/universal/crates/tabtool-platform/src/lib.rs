//! Platform services, settings and system locale for TabTool.

#![deny(unsafe_code)]

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
pub mod android;
mod bridge_result;
#[cfg(not(target_os = "android"))]
pub mod desktop;
pub mod locale;
pub mod services;
pub mod settings;
pub mod text_input;

#[cfg(not(target_os = "android"))]
pub use desktop::{DesktopPlatform, FileDialogs};
#[cfg(not(target_os = "android"))]
pub use locale::system_locale;
pub use locale::{SystemInfo, decimal_separator, prefers_chinese_encoding};
pub use services::{
    BackHandler, Clipboard, CopyOutcome, DocumentFiles, Insets, OpenUrl, PlatformError,
    PlatformErrors, Share,
};
pub use settings::{load_settings, save_settings, settings_path};
pub use text_input::{TextInput, TextInputOutcome, TextInputRequest};

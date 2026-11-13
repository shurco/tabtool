//! Desktop services: file dialogs, clipboard, opening URLs and system locale.

use std::path::PathBuf;

pub use crate::locale::{SystemInfo, SystemLocale, decimal_separator, system_locale};

/// The file dialogs a view needs. Tests use a fake; the app uses [`DesktopPlatform`].
pub trait FileDialogs {
    /// Asks for files to open; an empty list when the user cancels.
    fn open_files(&self) -> Vec<PathBuf>;
    /// Asks where to save a file with the suggested name and extension.
    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf>;
    /// Asks for a folder; `None` when the user cancels.
    fn choose_folder(&self) -> Option<PathBuf>;
}

/// The text clipboard.
pub trait Clipboard {
    /// The clipboard text, if there is any.
    fn get(&self) -> Option<String>;
    /// Replaces the clipboard text.
    fn set(&self, text: &str);
}

/// Opens a URL in the user's browser or mail client.
pub trait OpenUrl {
    /// Opens `url`, ignoring a failure as the Apple app does.
    fn open(&self, url: &str);
}

/// The real desktop services.
///
/// File dialogs use the platform's native panels. On Linux they go through
/// the XDG desktop portal (`rfd`'s `xdg-portal` backend), so the build needs
/// no GTK development packages and works in CI.
///
/// The clipboard is owned by egui in the desktop app: views call
/// `egui::Context::copy_text` and read paste events. [`DesktopPlatform`]'s
/// clipboard is a documented empty fallback so the trait also serves Android,
/// whose clipboard bridge is Plan 5.
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopPlatform;

impl FileDialogs for DesktopPlatform {
    fn open_files(&self) -> Vec<PathBuf> {
        rfd::FileDialog::new().pick_files().unwrap_or_default()
    }

    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf> {
        let language = tabtool_i18n::language_from_system(&SystemLocale.languages());
        rfd::FileDialog::new()
            .set_file_name(suggested_name)
            .add_filter(tabtool_i18n::tr(language, "CSV"), &[extension])
            .save_file()
    }

    fn choose_folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }
}

impl Clipboard for DesktopPlatform {
    /// The desktop app copies and pastes through egui; this fallback is empty.
    fn get(&self) -> Option<String> {
        None
    }

    /// The desktop app copies through egui; this fallback keeps no state.
    fn set(&self, _text: &str) {}
}

impl OpenUrl for DesktopPlatform {
    fn open(&self, url: &str) {
        let _ = open::that(url);
    }
}

impl SystemInfo for DesktopPlatform {
    fn languages(&self) -> Vec<String> {
        crate::locale::SystemLocale.languages()
    }

    fn locale(&self) -> String {
        system_locale()
    }

    fn decimal_separator(&self) -> String {
        decimal_separator(&system_locale())
    }
}

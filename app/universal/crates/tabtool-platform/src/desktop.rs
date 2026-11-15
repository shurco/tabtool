//! Desktop services: file dialogs, opening URLs and system locale.

use std::path::PathBuf;

use tabtool_core::TABLE_EXTENSIONS;

pub use crate::locale::{SystemInfo, decimal_separator, system_locale};

/// The file dialogs a view needs. Tests use a fake; the app uses [`DesktopPlatform`].
pub trait FileDialogs {
    /// Asks for files to open; an empty list when the user cancels.
    fn open_files(&self) -> Vec<PathBuf>;
    /// Asks where to save a file with the suggested name and extension.
    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf>;
    /// Asks for a folder; `None` when the user cancels.
    fn choose_folder(&self) -> Option<PathBuf>;
}

/// Opens a URL in the user's browser or mail client.
pub use crate::services::OpenUrl;

/// The real desktop services.
///
/// File dialogs use the platform's native panels. On Linux they go through
/// the XDG desktop portal (`rfd`'s `xdg-portal` backend), so the build needs
/// no GTK development packages and works in CI.
///
/// The clipboard is owned by egui in the desktop app: views call
/// `egui::Context::copy_text` and read paste events.
#[derive(Clone, Copy, Debug, Default)]
pub struct DesktopPlatform;

impl FileDialogs for DesktopPlatform {
    fn open_files(&self) -> Vec<PathBuf> {
        let language = tabtool_i18n::language_from_system(&self.languages());
        rfd::FileDialog::new()
            .add_filter(tabtool_i18n::tr(language, "Table Files"), &TABLE_EXTENSIONS)
            .pick_files()
            .unwrap_or_default()
    }

    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf> {
        let language = tabtool_i18n::language_from_system(&self.languages());
        rfd::FileDialog::new()
            .set_file_name(suggested_name)
            .add_filter(tabtool_i18n::tr(language, "Table Files"), &[extension])
            .save_file()
    }

    fn choose_folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }
}

impl OpenUrl for DesktopPlatform {
    fn open(&self, url: &str) {
        let _ = open::that(url);
    }
}

impl SystemInfo for DesktopPlatform {
    fn languages(&self) -> Vec<String> {
        sys_locale::get_locales().collect()
    }

    fn locale(&self) -> String {
        system_locale()
    }

    fn decimal_separator(&self) -> String {
        decimal_separator(&system_locale())
    }
}

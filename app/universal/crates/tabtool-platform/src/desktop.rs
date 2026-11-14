//! Desktop services: file dialogs, clipboard, opening URLs and system locale.

use std::path::{Path, PathBuf};

pub use crate::locale::{SystemInfo, SystemLocale, decimal_separator, system_locale};
use crate::services::{
    BackHandler, DocumentFiles, Insets, PlatformError, PlatformErrors, Share, TABLE_EXTENSIONS,
};
use crate::text_input::{TextInput, TextInputOutcome, TextInputRequest};

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
pub use crate::services::Clipboard;

/// Opens a URL in the user's browser or mail client.
pub use crate::services::OpenUrl;

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
        let language = tabtool_i18n::language_from_system(&SystemLocale.languages());
        rfd::FileDialog::new()
            .add_filter(tabtool_i18n::tr(language, "Table Files"), &TABLE_EXTENSIONS)
            .pick_files()
            .unwrap_or_default()
    }

    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf> {
        let language = tabtool_i18n::language_from_system(&SystemLocale.languages());
        rfd::FileDialog::new()
            .set_file_name(suggested_name)
            .add_filter(tabtool_i18n::tr(language, "Table Files"), &[extension])
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

impl TextInput for DesktopPlatform {
    /// Desktop views use egui's `TextEdit`; the native editor is Android-only.
    fn open(&self, _request: TextInputRequest) {}

    fn close(&self) {}

    fn poll(&self) -> Option<TextInputOutcome> {
        None
    }

    fn is_active(&self) -> bool {
        false
    }
}

impl DocumentFiles for DesktopPlatform {
    /// The user's Documents folder, as a placeholder until the desktop app
    /// wires its own document library.
    fn documents_dir(&self) -> PathBuf {
        directories::UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| std::env::temp_dir().join("TabTool"))
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        crate::services::table_files(&self.documents_dir())
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        let language = tabtool_i18n::language_from_system(&SystemLocale.languages());
        let source = rfd::FileDialog::new()
            .add_filter(tabtool_i18n::tr(language, "Table Files"), &TABLE_EXTENSIONS)
            .pick_file()?;
        let directory = self.documents_dir();
        std::fs::create_dir_all(&directory).ok()?;
        let name = source.file_name()?.to_string_lossy();
        let destination = unique_destination(&directory, &name);
        std::fs::copy(&source, &destination).ok()?;
        Some(destination)
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> bool {
        let Some(destination) = rfd::FileDialog::new()
            .set_file_name(suggested_name)
            .save_file()
        else {
            return false;
        };
        std::fs::copy(source, destination).is_ok()
    }

    fn delete(&self, path: &Path) -> std::io::Result<()> {
        crate::services::ensure_within(&self.documents_dir(), path)?;
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        let directory = self.documents_dir();
        crate::services::ensure_within(&directory, from)?;
        crate::services::ensure_within(&directory, to)?;
        std::fs::rename(from, to)
    }
}

impl PlatformErrors for DesktopPlatform {
    /// Desktop dialogs report failures directly; no queued platform error.
    fn take_error(&self) -> Option<PlatformError> {
        None
    }
}

impl Insets for DesktopPlatform {
    /// Desktop windows have no system bars and no soft keyboard.
    fn safe_area(&self) -> [f32; 4] {
        [0.0; 4]
    }

    fn keyboard_height(&self) -> f32 {
        0.0
    }
}

impl Share for DesktopPlatform {
    /// Desktop hands files to other apps by exporting them.
    fn share(&self, _path: &Path) {}
}

impl BackHandler for DesktopPlatform {
    /// The desktop app has no system back gesture.
    fn take_back(&self) -> bool {
        false
    }
}

/// `directory/name`, or `directory/name 2.ext` when that name is taken.
fn unique_destination(directory: &Path, name: &str) -> PathBuf {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(name).file_stem().map_or_else(
        || name.to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let extension = Path::new(name)
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    for index in 2.. {
        let candidate = directory.join(format!("{stem} {index}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("the loop returns on the first free name")
}

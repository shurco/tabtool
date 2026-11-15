//! Reading and writing the platform-neutral settings model.
//!
//! [`Settings`] itself lives in `tabtool-settings`; this crate owns the JSON
//! file, its atomic replace and its platform config-directory location.

mod encoding;
mod path_json;
mod persisted;

use std::path::{Path, PathBuf};

use tabtool_settings::Settings;

/// Loads the settings at `path`.
///
/// A missing or malformed file and every missing, mistyped or unknown value
/// fall back to the default, so a stale file can never reach the document or
/// the grid.
pub fn load_settings(path: &Path) -> Settings {
    persisted::load(path)
}

/// Atomically writes the settings to `path`.
pub fn save_settings(settings: &Settings, path: &Path) -> std::io::Result<()> {
    persisted::save(settings, path)
}

/// The settings file in the platform config directory.
///
/// The identifier matches the bundle id `app.shurco.tabtool`: on macOS that
/// yields `~/Library/Application Support/app.shurco.tabtool/settings.json`.
pub fn settings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("app", "shurco", "tabtool")
        .map(|directories| directories.config_dir().join("settings.json"))
}

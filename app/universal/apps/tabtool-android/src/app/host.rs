//! The platform seam the Android app runs on.

use std::path::{Path, PathBuf};

use tabtool_platform::{decimal_separator, load_settings, save_settings};
use tabtool_settings::Settings;
use tabtool_ui::mobile::{DocumentFiles, LibraryClock};

use crate::intents;

/// Everything the app needs from the platform except the live text and
/// clipboard seams, which are separate because the editor holds both at once.
pub trait Host: DocumentFiles {
    /// `[left, top, right, bottom]` safe-area insets in points.
    fn safe_area(&self) -> [f32; 4];
    /// The soft-keyboard height in points, or zero.
    fn keyboard_height(&self) -> f32;
    /// Takes a pending system back gesture.
    fn take_back(&self) -> bool;
    /// Shares a private document through the system share sheet.
    fn share(&self, path: &Path);
    /// Closes the activity.
    fn finish_activity(&self);
    /// The system language tags, most preferred first.
    fn languages(&self) -> Vec<String>;
    /// Whether the system is in night mode, when the platform can say.
    fn dark_mode(&self) -> Option<bool> {
        None
    }
    /// The system locale tag, for example `de_DE`.
    fn locale(&self) -> String;
    /// The device clock facts for file dates.
    fn clock(&self) -> LibraryClock {
        LibraryClock::default()
    }
    /// The locale's decimal separator.
    fn decimal_separator(&self) -> String {
        decimal_separator(&self.locale())
    }
    /// The settings file, if the platform has one.
    fn settings_path(&self) -> Option<PathBuf>;
    /// Reads the persisted settings.
    fn load_settings(&self) -> Settings {
        self.settings_path()
            .map_or_else(Settings::default, |path| load_settings(&path))
    }
    /// Persists the settings; a failure is logged and otherwise ignored.
    fn save_settings(&self, settings: &Settings) {
        if let Some(path) = self.settings_path()
            && let Err(error) = save_settings(settings, &path)
        {
            log::error!("could not save settings: {error}");
        }
    }
    /// The process-death state file, if the platform has one.
    fn state_path(&self) -> Option<PathBuf> {
        None
    }
    /// Takes one transition from the foreground to the background.
    fn take_background_event(&self) -> bool {
        false
    }
    /// Takes one document another app sent.
    fn take_intent(&self) -> Option<intents::IntentEvent> {
        None
    }
    /// Reports that a requested background save finished, for platforms that
    /// wait on the activity's pause path.
    fn background_save_finished(&self) {}
    /// Opens a web or mail link.
    fn open_url(&self, url: &str);
}

//! The app constructors and their injected storage, dialogs and settings path.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tabtool_platform::{DesktopPlatform, SystemInfo, prefers_chinese_encoding, settings_path};
use tabtool_settings::Settings;

use super::{DesktopApp, quit};
use crate::files::FileWorkflow;
use crate::platform::{SharedDialogs, SharedStorage, standard_dialogs, standard_storage};

impl DesktopApp {
    /// Creates an empty app with the filesystem storage.
    pub fn new(settings: Settings, platform: DesktopPlatform) -> Self {
        Self::build(
            settings,
            platform,
            standard_storage(),
            standard_dialogs(),
            settings_path(),
        )
    }

    /// Creates an empty app with an injected storage.
    pub fn with_storage(
        settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
    ) -> Self {
        Self::build(settings, platform, storage, standard_dialogs(), None)
    }

    /// Creates an empty app with injected storage and file dialogs.
    ///
    /// Tests use scripted dialogs so Save As and close paths run unattended.
    pub fn with_dialogs(
        settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
        dialogs: SharedDialogs,
    ) -> Self {
        Self::build(settings, platform, storage, dialogs, None)
    }

    /// Creates an empty app that persists settings at `path`.
    ///
    /// Tests use a temporary path so a preference change can be read back.
    pub fn with_settings_path(
        settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
        path: PathBuf,
    ) -> Self {
        Self::build(settings, platform, storage, standard_dialogs(), Some(path))
    }

    fn build(
        mut settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
        dialogs: SharedDialogs,
        settings_path: Option<PathBuf>,
    ) -> Self {
        let system_languages = platform.languages();
        let prefers_chinese = prefers_chinese_encoding(&system_languages);
        let language = settings.interface_language(&system_languages);
        let files = FileWorkflow::new(
            Arc::clone(&dialogs),
            Arc::clone(&storage),
            settings.clone(),
            settings_path.clone(),
            prefers_chinese,
        );
        // The workflow owns the canonical recent list; settings keeps a mirror
        // so the persisted JSON always matches what the UI shows.
        settings.recent_documents = files.recent().to_vec();
        Self {
            documents: Vec::new(),
            settings,
            about: false,
            settings_window: false,
            platform,
            files,
            storage,
            dialogs,
            settings_path,
            settings_window_state: None,
            settings_dirty: false,
            next_window: 0,
            untitled_count: 0,
            language,
            fonts_checked: false,
            system_languages,
            prefers_chinese,
            recent_picker: false,
            root_visible: None,
            about_closed: Arc::new(AtomicBool::new(false)),
            about_action: Arc::new(Mutex::new(None)),
            quit: quit::QuitState::default(),
        }
    }
}

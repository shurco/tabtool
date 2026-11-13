//! The mutable state and platform services of one document window.

use egui::ViewportId;
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::document::DocumentUiState;
use tabtool_ui::grid::GridState;

use crate::autosave::Autosave;
use crate::files::FilePolicy;
use crate::menu::MenuAction;
use crate::platform::{SharedDialogs, SharedStorage};

use super::{DocumentWindow, WindowRequest, export, open};

/// The platform services a document window uses.
pub struct WindowServices {
    /// Locale and decimal separators come from the platform.
    pub platform: DesktopPlatform,
    /// Scripted Save As panels in tests; the native panel in the app.
    pub dialogs: SharedDialogs,
    /// The shared document storage.
    pub storage: SharedStorage,
    /// The New Document format and large-file threshold.
    pub settings: Settings,
}

/// The mutable state of one document window.
pub(crate) struct WindowState {
    pub(crate) viewport_id: ViewportId,
    pub(crate) document: Document,
    pub(crate) untitled_name: String,
    pub(crate) grid: GridState,
    pub(crate) chrome: DocumentUiState,
    pub(crate) autosave: Autosave,
    pub(crate) file_ops: FilePolicy,
    pub(crate) language: Language,
    pub(crate) show_format_bar: bool,
    pub(crate) row_height: f32,
    pub(crate) alternating_rows: bool,
    pub(crate) shows_row_numbers: bool,
    pub(crate) has_recent: bool,
    pub(crate) locale: Locale,
    pub(crate) decimal_separator: String,
    pub(crate) requests: Vec<WindowRequest>,
    pub(crate) app_actions: Vec<MenuAction>,
    pub(crate) pending_actions: Vec<MenuAction>,
    pub(crate) close_prompt: bool,
    pub(crate) revert_prompt: bool,
    pub(crate) open: open::OpenState,
    pub(crate) export: export::ExportState,
    pub(crate) busy: bool,
    pub(crate) saving: bool,
    pub(crate) close_after_save: bool,
    pub(crate) should_close: bool,
    pub(crate) issue: Option<DocumentError>,
}

impl DocumentWindow {
    /// Updates the New Document format, large-file threshold and grid appearance.
    pub fn set_settings(&self, settings: Settings) {
        let mut state = self.lock();
        state.row_height = settings.row_height();
        state.alternating_rows = settings.grid_alternating_rows;
        state.shows_row_numbers = settings.grid_shows_row_numbers;
        state.file_ops.set_settings(settings);
    }
}

impl WindowState {
    /// The window title.
    pub(crate) fn title(&self) -> String {
        self.document
            .file_path()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.untitled_name.clone())
    }
}

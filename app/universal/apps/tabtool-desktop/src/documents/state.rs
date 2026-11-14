//! The mutable state and platform services of one document window.

use egui::ViewportId;
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings};
use tabtool_ui::document::{DocumentUiState, apply_grid_action};
use tabtool_ui::grid::{Clipboard, GridAction, GridAppearance, GridState};

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
    /// Whether the system language prefers GBK over CP1252.
    pub prefers_chinese: bool,
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
    pub(crate) appearance: GridAppearance,
    pub(crate) has_recent: bool,
    pub(crate) locale: Locale,
    pub(crate) decimal_separator: String,
    pub(crate) requests: Vec<WindowRequest>,
    pub(crate) app_actions: Vec<MenuAction>,
    pub(crate) prefers_chinese: bool,
    pub(crate) close_prompt: bool,
    pub(crate) revert_prompt: bool,
    pub(crate) open: open::OpenState,
    pub(crate) export: export::ExportState,
    pub(crate) busy: bool,
    pub(crate) saving: bool,
    pub(crate) close_after_save: bool,
    pub(crate) should_close: bool,
    pub(crate) issue: Option<DocumentError>,
    pub(crate) modal_blocked: bool,
}

impl DocumentWindow {
    /// Updates the New Document format, large-file threshold and grid appearance.
    pub fn set_settings(&self, settings: Settings) {
        let mut state = self.lock();
        state.appearance = GridAppearance {
            row_height: settings.row_height(),
            alternating_rows: settings.grid_alternating_rows,
            shows_row_numbers: settings.grid_shows_row_numbers,
        };
        state.file_ops.set_settings(settings);
    }

    /// The grid appearance the window is rendering with.
    pub fn grid_appearance(&self) -> GridAppearance {
        self.lock().appearance
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

    /// Writes any open cell or header editor before a command reads or changes
    /// the document.
    ///
    /// Close, quit, drop, revert and the menu all decide from
    /// [`Document::is_dirty`], but the editor text is not in the document yet.
    /// Committing here makes those decisions see the typed text and keeps it
    /// out of the next row when a structural command lands.
    pub(crate) fn commit_editor(&mut self) {
        if let Some(editing) = self.grid.editing.take() {
            let _ = apply_grid_action(
                &mut self.document,
                &mut self.grid,
                editing.commit_action(),
                &mut NoClipboard,
                &self.locale,
                &self.decimal_separator,
            );
        }
        let renamed = self
            .grid
            .header_editing
            .as_ref()
            .and_then(|editing| {
                self.document.row_source().and_then(|source| {
                    source
                        .column_ids()
                        .iter()
                        .position(|id| *id == editing.column)
                })
            })
            .and_then(|column| {
                self.grid
                    .header_editing
                    .take()
                    .map(|editing| GridAction::RenameColumn {
                        column,
                        name: editing.text,
                    })
            });
        if let Some(action) = renamed {
            let _ = apply_grid_action(
                &mut self.document,
                &mut self.grid,
                action,
                &mut NoClipboard,
                &self.locale,
                &self.decimal_separator,
            );
        }
    }

    /// Whether a prompt or modal owns the window's input.
    pub(crate) fn blocks_input(&self) -> bool {
        self.modal_blocked
            || self.close_prompt
            || self.revert_prompt
            || self.open.prompt
            || self.busy
            || (self.saving && (self.close_after_save || self.open.after_save.is_some()))
            || self.chrome.pending_reparse.is_some()
            || self.export.session.is_some()
            || self.export.confirmation
            || self.export.alert.is_some()
    }

    /// Whether autosave must wait.
    pub(crate) fn autosave_paused(&self) -> bool {
        self.blocks_input() || self.saving
    }
}

/// The editor commit never copies or pastes; a real clipboard is not needed.
struct NoClipboard;

impl Clipboard for NoClipboard {
    fn copy(&mut self, _text: &str) {}

    fn paste(&mut self) -> Option<String> {
        None
    }
}

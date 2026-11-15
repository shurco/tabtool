//! Document windows: one viewport, one [`Document`] and its close state.

mod alerts;
mod close;
mod commands;
mod export;
mod grid;
mod messages;
mod open;
mod requests;
mod state;
mod view;

pub use grid::EguiClipboard;
pub(crate) use messages::{issue_message, saving_text};
pub use state::WindowServices;

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use egui::{Context, Ui, ViewportId};
use tabtool_document::{Autosave, Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_ui::document::{DocumentUiState, sync_selection_to_grid};
use tabtool_ui::grid::GridState;

use crate::files::FilePolicy;
use crate::menu::MenuAction;
use crate::platform::{decimal_separator, system_locale};
pub use close::{CloseDecision, CloseSave, close_decision};
pub use requests::CloseChoice;
pub(crate) use requests::WindowRequest;
pub(crate) use state::WindowState;

/// The Apple app's minimum document window size.
pub const DOCUMENT_MIN_INNER_SIZE: [f32; 2] = [640.0, 400.0];

/// One document viewport.
///
/// The value is cheap to clone: the document and UI state live behind a shared
/// mutex so a deferred viewport callback can own a handle while the app keeps
/// the same window in its list.
#[derive(Clone)]
pub struct DocumentWindow {
    viewport_id: ViewportId,
    state: Arc<Mutex<WindowState>>,
}

impl DocumentWindow {
    /// Creates a window for `document` at `viewport_id`.
    pub fn new(
        viewport_id: ViewportId,
        document: Document,
        untitled_name: String,
        language: Language,
        show_format_bar: bool,
        services: WindowServices,
    ) -> Self {
        let WindowServices {
            platform,
            dialogs,
            storage,
            settings,
            prefers_chinese,
        } = services;
        let autosave = Autosave::new(&document);
        let viewport_id_for_state = viewport_id;
        let appearance = tabtool_ui::grid::GridAppearance {
            row_height: settings.row_height(),
            alternating_rows: settings.grid_alternating_rows,
            shows_row_numbers: settings.grid_shows_row_numbers,
        };
        let state = WindowState {
            viewport_id: viewport_id_for_state,
            document,
            untitled_name,
            grid: GridState::default(),
            chrome: DocumentUiState::default(),
            autosave,
            file_ops: FilePolicy::new(dialogs, storage, settings, prefers_chinese),
            language,
            show_format_bar,
            appearance,
            has_recent: false,
            locale: system_locale(&platform),
            decimal_separator: decimal_separator(&platform),
            requests: Vec::new(),
            app_actions: Vec::new(),
            prefers_chinese,
            close_prompt: false,
            revert_prompt: false,
            open: open::OpenState::default(),
            export: export::ExportState::default(),
            busy: false,
            saving: false,
            close_after_save: false,
            should_close: false,
            issue: None,
            modal_blocked: false,
        };
        Self {
            viewport_id,
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// The native viewport this window owns.
    pub fn viewport_id(&self) -> ViewportId {
        self.viewport_id
    }

    /// The window title: the file name, or the numbered untitled name.
    pub fn title(&self) -> String {
        self.lock().title()
    }

    /// Reads the document behind the window.
    pub fn with_document<T>(&self, read: impl FnOnce(&Document) -> T) -> T {
        read(&self.lock().document)
    }

    /// The document's unsaved state.
    ///
    /// An open editor counts as unsaved until its text is in the document.
    pub fn is_dirty(&self) -> bool {
        let mut state = self.lock();
        state.commit_editor();
        state.document.is_dirty()
    }

    /// What closing the window should do now.
    pub fn close_decision(&self) -> CloseDecision {
        let mut state = self.lock();
        state.commit_editor();
        close_decision(&state.document)
    }

    /// Whether a prompt or modal owns this window's input.
    pub fn has_modal(&self) -> bool {
        self.lock().blocks_input()
    }

    /// Blocks this window's shortcuts and grid keys while the app-level quit
    /// prompt or progress alert is visible.
    pub fn set_modal_blocked(&self, blocked: bool) {
        self.lock().modal_blocked = blocked;
    }

    /// Whether the window has finished closing and should leave the app list.
    pub fn should_close(&self) -> bool {
        self.lock().should_close
    }

    /// Updates the interface language and the global View settings.
    pub fn set_chrome(&self, language: Language, show_format_bar: bool, has_recent: bool) {
        let mut state = self.lock();
        state.language = language;
        state.show_format_bar = show_format_bar;
        state.has_recent = has_recent;
    }

    /// Runs the window's pending requests, autosave and save polling.
    ///
    /// Returns the app-level menu actions the window's menu produced.
    pub fn tick(&self, now: Instant, ctx: &Context) -> Vec<MenuAction> {
        let mut guard = self.lock();
        let state = &mut *guard;
        state.drain_requests(ctx);
        if state.should_close {
            // The user chose not to save; autosave and save polling must not
            // write the edits the close discarded.
            return std::mem::take(&mut state.app_actions);
        }

        sync_selection_to_grid(&state.document, &mut state.grid);
        // A blocked autosave records the typed write issue. Undoing the edit
        // that caused it makes the gate clean, so the stale error must not
        // surface as a failed save.
        if matches!(state.issue, Some(DocumentError::Csv(_)))
            && state.document.write_issue().is_none()
        {
            state.issue = None;
        }

        let outcome = if state.autosave_paused() {
            tabtool_document::AutosaveOutcome::Idle
        } else {
            state
                .autosave
                .tick(now, &mut state.document, &*state.file_ops.storage)
        };
        match outcome {
            tabtool_document::AutosaveOutcome::Saved => state.issue = None,
            tabtool_document::AutosaveOutcome::Blocked(error) => state.issue = Some(error),
            _ => {}
        }

        state.poll_save();
        std::mem::take(&mut state.app_actions)
    }

    /// How long until this window needs another pass.
    pub fn repaint_after(&self, now: Instant) -> Option<Duration> {
        let state = self.lock();
        if state.saving {
            return Some(Duration::ZERO);
        }
        state.autosave.repaint_after(now)
    }

    /// The progress of a running large save, from 0 to 1.
    pub fn save_progress(&self) -> Option<f64> {
        let state = self.lock();
        if state.saving {
            state.document.large_save_progress()
        } else {
            None
        }
    }

    /// Whether a large save is running.
    pub fn is_saving(&self) -> bool {
        self.lock().saving
    }

    /// Asks the window to run its close policy.
    ///
    /// This is what a native close request and File > Close use. The close
    /// alert, or an immediate close, is visible to the renderer.
    pub fn request_close(&self) {
        let mut state = self.lock();
        state.commit_editor();
        match close_decision(&state.document) {
            CloseDecision::Close => state.should_close = true,
            CloseDecision::Ask => state.close_prompt = true,
            CloseDecision::Busy => state.busy = true,
        }
    }

    /// Whether the unsaved-changes alert is showing.
    pub fn has_close_prompt(&self) -> bool {
        self.lock().close_prompt
    }

    /// Answers the unsaved-changes alert; the next tick performs the choice.
    pub fn answer_close_prompt(&self, choice: CloseChoice) {
        self.lock().requests.push(WindowRequest::Close(choice));
    }

    /// Whether a completed large save will close this window.
    pub fn is_closing_after_save(&self) -> bool {
        self.lock().close_after_save
    }

    /// Stops a pending close without cancelling the large save.
    pub(crate) fn abort_close_after_save(&self) {
        self.lock().close_after_save = false;
    }

    /// Saves the document for a close or quit.
    pub fn begin_close_save(&self) -> CloseSave {
        self.lock().save_for_close()
    }

    /// Polls a large close save; `None` while it is still running.
    pub fn poll_close_save(&self) -> Option<Result<(), DocumentError>> {
        self.lock().poll_save()
    }

    /// Cancels a large close save and aborts the close.
    pub fn cancel_close_save(&self) {
        self.lock().cancel_save();
    }

    /// Renders the window.
    pub fn render(&self, ui: &mut Ui) {
        self.lock().render(ui);
    }

    fn lock(&self) -> MutexGuard<'_, WindowState> {
        self.state
            .lock()
            .expect("the document window lock is not poisoned")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use egui::ViewportId;
    use tabtool_core::CsvConfiguration;
    use tabtool_document::{Document, StdStorage};
    use tabtool_i18n::Language;
    use tabtool_platform::{DesktopPlatform, FileDialogs};
    use tabtool_settings::Settings;

    use super::{DocumentWindow, WindowServices};

    struct NoDialogs;

    impl FileDialogs for NoDialogs {
        fn open_files(&self) -> Vec<PathBuf> {
            Vec::new()
        }

        fn save_file(&self, _suggested_name: &str, _extension: &str) -> Option<PathBuf> {
            None
        }

        fn choose_folder(&self) -> Option<PathBuf> {
            None
        }
    }

    fn window() -> DocumentWindow {
        let configuration = CsvConfiguration::default();
        let mut document = Document::new(configuration.clone());
        document
            .new_document(configuration)
            .expect("create the empty document");
        DocumentWindow::new(
            ViewportId::ROOT,
            document,
            "Untitled".to_owned(),
            Language::En,
            true,
            WindowServices {
                platform: DesktopPlatform,
                dialogs: Arc::new(NoDialogs),
                storage: Arc::new(StdStorage),
                settings: Settings::default(),
                prefers_chinese: false,
            },
        )
    }

    #[test]
    fn every_prompt_modal_and_pending_reparse_pauses_autosave() {
        let window = window();
        let mut state = window.lock();
        assert!(!state.autosave_paused());

        state.close_prompt = true;
        assert!(state.autosave_paused(), "close prompt");
        state.close_prompt = false;

        state.revert_prompt = true;
        assert!(state.autosave_paused(), "revert prompt");
        state.revert_prompt = false;

        state.open.prompt = true;
        assert!(state.autosave_paused(), "open prompt");
        state.open.prompt = false;

        state.busy = true;
        assert!(state.autosave_paused(), "busy alert");
        state.busy = false;

        state.saving = true;
        assert!(state.autosave_paused(), "running save");
        state.saving = false;

        state.modal_blocked = true;
        assert!(state.autosave_paused(), "app-level quit prompt");
        state.modal_blocked = false;

        state.chrome.pending_reparse = Some(CsvConfiguration::default());
        assert!(state.autosave_paused(), "pending reparse");
        state.chrome.pending_reparse = None;

        state.export.session = state.file_ops.export_session(&state.document);
        assert!(state.export.session.is_some());
        assert!(state.autosave_paused(), "export panel");
        state.export.session = None;

        state.export.confirmation = true;
        assert!(state.autosave_paused(), "export confirmation");
        state.export.confirmation = false;

        state.export.alert = Some(super::export::ExportAlert {
            title: "Export Not Completed",
            message: String::new(),
        });
        assert!(state.autosave_paused(), "export alert");
        state.export.alert = None;

        assert!(!state.autosave_paused());
    }
}

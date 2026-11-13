//! Document windows: one viewport, one [`Document`] and its close state.

mod alerts;
mod close;
mod commands;
mod grid;
mod messages;
mod requests;
mod view;

pub use grid::EguiClipboard;
pub(crate) use messages::{issue_message, saving_text};

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use egui::{Context, Ui, ViewportId};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_ui::document::{DocumentUiState, sync_selection_to_grid};
use tabtool_ui::grid::GridState;

use crate::autosave::Autosave;
use crate::menu::MenuAction;
use crate::platform::{SharedDialogs, SharedStorage, decimal_separator, system_locale};
pub use close::{CloseDecision, CloseSave, close_decision};
pub use requests::CloseChoice;
pub(crate) use requests::WindowRequest;

/// The Apple app's minimum document window size.
pub const DOCUMENT_MIN_INNER_SIZE: [f32; 2] = [640.0, 400.0];

/// The platform services a document window uses.
pub struct WindowServices {
    /// Locale and decimal separators come from the platform.
    pub platform: DesktopPlatform,
    /// Scripted Save As panels in tests; the native panel in the app.
    pub dialogs: SharedDialogs,
    /// The shared document storage.
    pub storage: SharedStorage,
}

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
        let autosave = Autosave::new(&document);
        let viewport_id_for_state = viewport_id;
        let state = WindowState {
            viewport_id: viewport_id_for_state,
            document,
            untitled_name,
            grid: GridState::default(),
            chrome: DocumentUiState::default(),
            autosave,
            storage: services.storage,
            dialogs: services.dialogs,
            language,
            show_format_bar,
            has_recent: false,
            locale: system_locale(&services.platform),
            decimal_separator: decimal_separator(&services.platform),
            requests: Vec::new(),
            app_actions: Vec::new(),
            pending_actions: Vec::new(),
            close_prompt: false,
            revert_prompt: false,
            busy: false,
            saving: false,
            close_after_save: false,
            should_close: false,
            issue: None,
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

    /// The document's unsaved state.
    pub fn is_dirty(&self) -> bool {
        self.lock().document.is_dirty()
    }

    /// What closing the window should do now.
    pub fn close_decision(&self) -> CloseDecision {
        close_decision(&self.lock().document)
    }

    /// Whether the window has finished closing and should leave the app list.
    pub fn should_close(&self) -> bool {
        self.lock().should_close
    }

    /// Actions the shell (or a later task) still has to handle.
    pub fn pending_actions(&self) -> Vec<MenuAction> {
        self.lock().pending_actions.clone()
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

        let outcome = if state.close_prompt || state.revert_prompt || state.saving {
            crate::autosave::AutosaveOutcome::Idle
        } else {
            state
                .autosave
                .tick(now, &mut state.document, &*state.storage)
        };
        match outcome {
            crate::autosave::AutosaveOutcome::Saved => state.issue = None,
            crate::autosave::AutosaveOutcome::Blocked(error) => state.issue = Some(error),
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

/// The mutable state of one document window.
pub(crate) struct WindowState {
    pub(crate) viewport_id: ViewportId,
    pub(crate) document: Document,
    pub(crate) untitled_name: String,
    pub(crate) grid: GridState,
    pub(crate) chrome: DocumentUiState,
    pub(crate) autosave: Autosave,
    pub(crate) storage: SharedStorage,
    pub(crate) dialogs: SharedDialogs,
    pub(crate) language: Language,
    pub(crate) show_format_bar: bool,
    pub(crate) has_recent: bool,
    pub(crate) locale: Locale,
    pub(crate) decimal_separator: String,
    pub(crate) requests: Vec<WindowRequest>,
    pub(crate) app_actions: Vec<MenuAction>,
    pub(crate) pending_actions: Vec<MenuAction>,
    pub(crate) close_prompt: bool,
    pub(crate) revert_prompt: bool,
    pub(crate) busy: bool,
    pub(crate) saving: bool,
    pub(crate) close_after_save: bool,
    pub(crate) should_close: bool,
    pub(crate) issue: Option<DocumentError>,
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

//! Document windows: one viewport, one [`Document`] and its close state.

mod close;
mod commands;
mod messages;
mod view;

pub(crate) use messages::{issue_message, saving_text};

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use egui::{Context, Ui, ViewportId};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, LargeStatus};
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;

use crate::autosave::Autosave;
use crate::menu::MenuAction;
use crate::platform::{SharedStorage, decimal_separator, system_locale};

/// The Apple app's minimum document window size.
pub const DOCUMENT_MIN_INNER_SIZE: [f32; 2] = [640.0, 400.0];

/// The grid engine state.
///
/// Task 4 replaces this empty placeholder with the virtualized grid state; the
/// shell only needs the field to exist and survive a repaint.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridState;

/// What closing a document should do now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseDecision {
    /// Clean; close without asking.
    Close,
    /// Dirty; ask Save / Don't Save / Cancel.
    Ask,
    /// A save owns the document; do not close yet.
    Busy,
}

/// Decides whether closing `document` needs a prompt.
///
/// A running large save wins over the dirty check: the document cannot be
/// closed until the saver settles. An untitled document is dirty until Save As
/// binds a path, exactly like the Apple app.
pub fn close_decision(document: &Document) -> CloseDecision {
    if matches!(document.large_status(), LargeStatus::Saving(_)) {
        CloseDecision::Busy
    } else if document.is_dirty() {
        CloseDecision::Ask
    } else {
        CloseDecision::Close
    }
}

/// How an explicit save-for-close ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseSave {
    /// The bytes are on disk; the window may close.
    Saved,
    /// A large streaming save is running; poll it before closing.
    Saving,
    /// The user cancelled the save panel; the close is cancelled.
    Cancelled,
    /// The save failed; the window stays open and the issue is visible.
    Failed(DocumentError),
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
        platform: DesktopPlatform,
        storage: SharedStorage,
    ) -> Self {
        let autosave = Autosave::new(&document);
        let state = WindowState {
            document,
            untitled_name,
            grid: GridState,
            autosave,
            storage,
            platform,
            language,
            show_format_bar,
            has_recent: false,
            locale: system_locale(&platform),
            decimal_separator: decimal_separator(&platform),
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

    /// The grid state; Task 4 replaces the placeholder type.
    pub fn grid(&self) -> GridState {
        self.lock().grid
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

/// A request the window UI makes to the state machine.
pub(crate) enum WindowRequest {
    /// A menu item was chosen.
    Menu(MenuAction),
    /// The user answered the unsaved-changes alert.
    Close(CloseChoice),
    /// The user answered the revert alert; `true` confirms.
    Revert(bool),
    /// The user cancelled a large close save.
    CancelSave,
}

/// The unsaved-changes alert choices.
pub(crate) enum CloseChoice {
    /// Save, then close.
    Save,
    /// Close and discard the edits.
    DontSave,
    /// Keep the window open.
    Cancel,
}

/// The mutable state of one document window.
pub(crate) struct WindowState {
    pub(crate) document: Document,
    pub(crate) untitled_name: String,
    pub(crate) grid: GridState,
    pub(crate) autosave: Autosave,
    pub(crate) storage: SharedStorage,
    pub(crate) platform: DesktopPlatform,
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

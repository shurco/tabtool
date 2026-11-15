//! The single-window Android app: navigation, autosave, intents and sheets.
//!
//! The view composes the shared mobile primitives. Every platform call goes
//! through [`Host`] (plus the live text-input and clipboard seams), so the
//! whole lifecycle is testable without an Android device.

mod actions;
mod poll;
mod render;

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use eframe::egui::{Frame, Panel, Ui, UiBuilder};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, LargeStatus, StdStorage};
use tabtool_i18n::{ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE, Language, tr};
use tabtool_platform::{decimal_separator, load_settings, prefers_chinese_encoding, save_settings};
use tabtool_settings::Settings;
use tabtool_ui::about::AboutAction;
use tabtool_ui::grid::{Clipboard as GridClipboard, GridAppearance, GridState};
use tabtool_ui::mobile::{
    CopyResult, DocumentMetadata, ExportOutcome, ExportSheet, FiltersSheet, FormatSheet,
    LibraryClock, LibraryError, LibraryResponse, LibraryView, MobileEditor, MobileEditorResponse,
    MobileEditorState, MobileMetrics, NavStack, OverflowItem, SaveCopy, Screen, SheetKind,
    SheetStyle, about_sheet, saved_copy_message, settings_sheet,
};
use tabtool_ui::theme::current;
use tabtool_ui::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

use crate::intents;
use crate::library;
use crate::lifecycle;
use crate::nav::{self, NavSnapshot};

/// The project website the About sheet opens.
pub const WEBSITE_URL: &str = "https://tabtool.shurco.app";
/// The MIT license bundled beside the workspace.
pub const LICENSE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../LICENSE"));

/// Everything the app needs from the platform except the live text and
/// clipboard seams, which are separate because the editor holds both at once.
pub trait Host {
    /// The app's private documents directory.
    fn documents_dir(&self) -> PathBuf;
    /// The supported documents in the private directory.
    fn list_documents(&self) -> Vec<PathBuf>;
    /// Every file name in the private directory, including hidden staging.
    fn existing_names(&self) -> Vec<String>;
    /// Creates a document without replacing an existing file.
    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf>;
    /// Asks the user to pick a file and copies it into the private directory.
    fn import_from_picker(&self) -> Option<PathBuf>;
    /// Whether an import picker started earlier is still open.
    fn import_in_progress(&self) -> bool {
        false
    }
    /// Copies `source` to a location the user chooses.
    fn export_copy(&self, source: &Path, suggested_name: &str) -> ExportOutcome;
    /// Whether an export picker started earlier is still open.
    fn export_in_progress(&self) -> bool {
        false
    }
    /// Deletes a private document.
    fn delete(&self, path: &Path) -> io::Result<()>;
    /// Renames a private document.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// Takes the next platform failure.
    fn take_error(&self) -> Option<LibraryError>;
    /// Reads a document's metadata.
    fn metadata(&self, path: &Path) -> io::Result<DocumentMetadata>;
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

/// A one-shot banner the app shows above the screen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    /// A localized failure, by catalog key.
    Error(&'static str),
    /// A ready-made informational message.
    Message(String),
}

/// What the app should do after the prompt's save or discard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AfterSaveAction {
    /// Close the editor and return to the library.
    CloseEditor,
    /// Open this document.
    Open(PathBuf),
    /// Share the current document.
    Share,
}

/// The Save / Don't Save / Cancel prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavePrompt {
    /// What happens when the user saves or discards.
    pub action: AfterSaveAction,
    /// The document name the prompt shows.
    pub name: String,
}

/// The user's answer to the unsaved-changes prompt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptChoice {
    /// Write the document, then continue.
    Save,
    /// Discard the in-memory edits, then continue.
    DontSave,
    /// Stay where the user was.
    Cancel,
}

/// What one back gesture did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackResult {
    /// A sheet closed.
    ClosedSheet,
    /// The editor closed and the library is showing.
    ClosedDocument,
    /// A prompt is showing.
    Prompt,
    /// A commit is pending; the back runs after it lands.
    Deferred,
    /// The app kept the screen, for example while a save runs.
    Ignored,
    /// The prompt was cancelled.
    CancelledPrompt,
    /// The library is at the root, so the activity exits.
    Exit,
}

/// The work that waits for an open native edit to commit.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Pending {
    Back,
}

/// One open document and its view state.
pub struct Editor {
    document: Document,
    grid: GridState,
    state: MobileEditorState,
    autosave: tabtool_document::Autosave,
    filters: FiltersSheet,
    format: FormatSheet,
    export: Option<ExportSheet>,
    save_copy: SaveCopy,
    title: String,
    /// What to do after a large save finishes.
    pending_after_save: Option<AfterSaveAction>,
}

impl Editor {
    fn new(document: Document, path: &Path) -> Self {
        let autosave = tabtool_document::Autosave::new(&document);
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        Self {
            document,
            grid: GridState::default(),
            state: MobileEditorState::default(),
            autosave,
            filters: FiltersSheet::new(),
            format: FormatSheet::new(),
            export: None,
            save_copy: SaveCopy::new(),
            title,
            pending_after_save: None,
        }
    }

    /// The open document.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The open document, for a direct edit in tests and platform callbacks.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The document name shown in the toolbar.
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// The layout key that forces a new library view when it changes.
#[derive(Clone, Debug, PartialEq)]
struct LibraryKey {
    language: Language,
    metrics: MobileMetrics,
    configuration: tabtool_core::CsvConfiguration,
}

/// The Android application.
pub struct AndroidApp {
    host: Rc<dyn Host>,
    files: library::AppFiles,
    text_input: Box<dyn tabtool_ui::mobile::TouchTextInput>,
    clipboard: Box<dyn GridClipboard>,
    settings: Settings,
    languages: Vec<String>,
    prefers_chinese: bool,
    language: Language,
    locale: Locale,
    decimal_separator: String,
    clock: LibraryClock,
    nav: NavStack,
    library: LibraryView<'static>,
    library_key: LibraryKey,
    editor: Option<Editor>,
    notice: Option<Notice>,
    prompt: Option<SavePrompt>,
    pending: Option<Pending>,
    background_pending: bool,
}

impl AndroidApp {
    /// Creates the app, loads its settings and restores the last session.
    pub fn new(
        host: Rc<dyn Host>,
        text_input: Box<dyn tabtool_ui::mobile::TouchTextInput>,
        clipboard: Box<dyn GridClipboard>,
    ) -> Self {
        let settings = host.load_settings();
        let languages = host.languages();
        let prefers_chinese = prefers_chinese_encoding(&languages);
        let language = settings.interface_language(&languages);
        let locale = library::parse_locale(&host.locale());
        let decimal_separator = host.decimal_separator();
        let clock = host.clock();
        let metrics = metrics_for(
            MobileMetrics::phone(),
            &settings,
            tabtool_ui::mobile::Insets::from_array(host.safe_area()),
        );
        let library = library::new_view(
            Rc::clone(&host),
            &settings,
            locale.clone(),
            clock,
            metrics,
            language,
        );
        let library_key = LibraryKey {
            language,
            metrics,
            configuration: settings.new_document_configuration(),
        };
        let snapshot = lifecycle::load_state(host.state_path().as_deref());
        let removed = intents::clean_stale(&host.documents_dir(), intents::STALE_STAGING_AGE);
        if removed > 0 {
            log::info!("removed {removed} abandoned import staging files");
        }
        let mut app = Self {
            files: library::AppFiles::new(Rc::clone(&host)),
            host,
            text_input,
            clipboard,
            settings,
            languages,
            prefers_chinese,
            language,
            locale,
            decimal_separator,
            clock,
            nav: NavStack::new(),
            library,
            library_key,
            editor: None,
            notice: None,
            prompt: None,
            pending: None,
            background_pending: false,
        };
        app.restore(&snapshot);
        app
    }

    /// The navigation stack.
    pub fn nav(&self) -> &NavStack {
        &self.nav
    }

    /// The open document editor, if any.
    pub fn editor(&self) -> Option<&Editor> {
        self.editor.as_ref()
    }

    /// The open document editor, if any.
    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        self.editor.as_mut()
    }

    /// The banner currently showing.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// The unsaved-changes prompt currently showing.
    pub fn prompt(&self) -> Option<&SavePrompt> {
        self.prompt.as_ref()
    }

    /// The persisted settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The resolved interface language.
    pub fn language(&self) -> Language {
        self.language
    }

    /// The root screen.
    pub fn screen(&self) -> &Screen {
        self.nav.screen()
    }
}

fn metrics_for(
    base: MobileMetrics,
    settings: &Settings,
    safe_area: tabtool_ui::mobile::Insets,
) -> MobileMetrics {
    base.with_row_height(settings.grid_row_height)
        .with_safe_area(safe_area)
}

/// The native editor rectangle in content-view coordinates.
///
/// The app draws inside the safe area, so the platform editor's top-left
/// corner must move by the content origin.
pub fn text_input_rect(rect: eframe::egui::Rect, safe_area: [f32; 4]) -> [f32; 4] {
    [
        rect.min.x + safe_area[0].max(0.0),
        rect.min.y + safe_area[1].max(0.0),
        rect.width(),
        rect.height(),
    ]
}

/// Maps the platform's copy result onto the UI's export outcome.
///
/// A pending picker is reported as [`ExportOutcome::Cancelled`] so the sheet
/// keeps waiting; a finished `false` without a typed outcome is a failure.
pub fn export_outcome(
    copied: bool,
    pending: bool,
    outcome: Option<tabtool_platform::CopyOutcome>,
) -> ExportOutcome {
    match outcome {
        Some(tabtool_platform::CopyOutcome::Copied) => ExportOutcome::Copied,
        Some(tabtool_platform::CopyOutcome::Cancelled) => ExportOutcome::Cancelled,
        Some(tabtool_platform::CopyOutcome::Failed) => ExportOutcome::Failed,
        None if pending => ExportOutcome::Cancelled,
        None if copied => ExportOutcome::Copied,
        None => ExportOutcome::Failed,
    }
}

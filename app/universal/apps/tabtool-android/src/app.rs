//! The single-window Android app: navigation, autosave, intents and sheets.
//!
//! The view composes the shared mobile primitives. Every platform call goes
//! through [`Host`] (plus the live text-input and clipboard seams), so the
//! whole lifecycle is testable without an Android device.

mod actions;
mod host;
mod poll;
mod prompt;
mod render;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use eframe::egui::{Frame, Panel, Ui, UiBuilder};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, LargeStatus, StdStorage};
use tabtool_i18n::{ERROR_GENERIC, ERROR_OPEN, ERROR_SAVE, Language, tr};
use tabtool_platform::{parse_locale, prefers_chinese_encoding};
use tabtool_settings::Settings;
use tabtool_ui::about::AboutAction;
use tabtool_ui::grid::{Clipboard as GridClipboard, GridAppearance, GridState};
use tabtool_ui::mobile::{
    CopyResult, ExportOutcome, ExportSheet, FiltersSheet, FormatSheet, LibraryClock,
    LibraryResponse, LibraryView, MobileEditor, MobileEditorResponse, MobileEditorState,
    MobileMetrics, NavStack, OverflowItem, SaveCopy, Screen, SheetKind, SheetStyle, about_sheet,
    saved_copy_message, settings_sheet,
};
use tabtool_ui::theme::current;
use tabtool_ui::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

use crate::intents;
use crate::library;
use crate::lifecycle;
use crate::nav::{self, NavSnapshot};

pub use host::Host;
pub use prompt::{AfterSaveAction, BackResult, Notice, PromptChoice, SavePrompt};

/// The project website the About sheet opens.
pub const WEBSITE_URL: &str = "https://tabtool.shurco.app";
/// The MIT license bundled beside the workspace.
pub const LICENSE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../LICENSE"));

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
    library: LibraryView,
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
        let locale = parse_locale(&host.locale());
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
/// keeps waiting; a finished call without a typed outcome is a failure.
pub fn export_outcome(
    outcome: Option<tabtool_platform::CopyOutcome>,
    pending: bool,
) -> ExportOutcome {
    match outcome {
        Some(tabtool_platform::CopyOutcome::Copied) => ExportOutcome::Copied,
        Some(tabtool_platform::CopyOutcome::Cancelled) => ExportOutcome::Cancelled,
        Some(tabtool_platform::CopyOutcome::Failed) => ExportOutcome::Failed,
        None if pending => ExportOutcome::Cancelled,
        None => ExportOutcome::Failed,
    }
}

//! The single-window Android app: navigation, autosave, intents and sheets.
//!
//! The view composes the shared mobile primitives. Every platform call goes
//! through [`Host`] (plus the live text-input and clipboard seams), so the
//! whole lifecycle is testable without an Android device.

use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use eframe::egui::{Frame, Panel, Ui, UiBuilder};
use tabtool_core::Locale;
use tabtool_document::{Document, DocumentError, LargeStatus, StdStorage};
use tabtool_i18n::{Language, language_from_system, tr};
use tabtool_platform::{Settings, decimal_separator, load_settings, save_settings};
use tabtool_ui::about::AboutAction;
use tabtool_ui::grid::{Clipboard as GridClipboard, GridAppearance, GridState};
use tabtool_ui::mobile::{
    CopyResult, DocumentMetadata, ExportOutcome, ExportSheet, FiltersSheet, FormatSheet,
    LibraryClock, LibraryError, LibraryResponse, LibraryView, MobileEditor, MobileEditorResponse,
    MobileEditorState, MobileMetrics, OverflowItem, SaveCopy, Screen, SheetKind, SheetStyle,
    about_sheet, saved_copy_message, settings_sheet,
};
use tabtool_ui::theme::current;
use tabtool_ui::widgets::{AlertAction, BannerKind, ButtonKind, banner, modal};

use crate::intents;
use crate::library;
use crate::lifecycle::{self, BACKGROUND_LARGE_WAIT};
use crate::nav::{AndroidNav, NavSnapshot};

/// The catalog key of a generic platform failure.
pub const ERROR_GENERIC: &str = "Something Went Wrong";
/// The catalog key of a document that could not be opened.
pub const ERROR_OPEN: &str = "The file could not be opened";
/// The catalog key of a document that could not be saved.
pub const ERROR_SAVE: &str = "The file could not be saved";
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

/// What a back gesture should do after a prompt is answered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseAction {
    /// Close the editor and return to the library.
    CloseEditor,
    /// Save, then open this document.
    Open(PathBuf),
}

/// The Save / Don't Save / Cancel prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavePrompt {
    /// What happens when the user saves or discards.
    pub action: CloseAction,
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
    pending_after_save: Option<CloseAction>,
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
    nav: AndroidNav,
    library: LibraryView<'static>,
    library_key: LibraryKey,
    editor: Option<Editor>,
    notice: Option<Notice>,
    prompt: Option<SavePrompt>,
    pending: Option<Pending>,
    background_wait: Option<Instant>,
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
        let prefers_chinese = language_from_system(&languages) == Language::ZhHans;
        let language = library::preferred_language(&settings, &languages);
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
            nav: AndroidNav::new(),
            library,
            library_key,
            editor: None,
            notice: None,
            prompt: None,
            pending: None,
            background_wait: None,
            background_pending: false,
        };
        app.restore(&snapshot);
        app
    }

    /// The navigation stack.
    pub fn nav(&self) -> &AndroidNav {
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

    /// Advances time, platform events and autosave without drawing.
    pub fn poll(&mut self, now: Instant) {
        self.sync_nav_notice();
        self.poll_platform_error();
        self.poll_intents();
        if self.host.take_back() {
            let _ = self.back();
        }
        if self.host.take_background_event() {
            self.background_pending = true;
            self.persist();
        }
        self.poll_editor(now);
        self.poll_pending();
        self.flush_background(now);
    }

    /// Draws one frame of the app.
    pub fn show(&mut self, ui: &mut Ui) {
        self.poll(Instant::now());
        let language = self.refresh_language();
        let metrics = self.metrics(ui.available_width());
        self.ensure_library(metrics, language);

        let background = current(ui.ctx()).colors.window_background;
        ui.painter().rect_filled(ui.max_rect(), 0.0, background);
        let content_rect = metrics.content_rect(ui.max_rect());
        let mut content =
            ui.new_child(UiBuilder::new().max_rect(content_rect).layout(*ui.layout()));
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));

        self.draw_notice(&mut content, language);
        match self.nav.screen() {
            Screen::Library => self.show_library(&mut content, metrics, language),
            Screen::Editor(_) => self.show_editor(&mut content, metrics, language),
        }
        // A background request that arrived while a native edit was open saves
        // after the editor has applied the commit.
        self.flush_background(Instant::now());
        self.show_sheets(&mut content, metrics, language);
        self.show_prompt(&mut content, language);
    }

    /// Handles one back gesture from the toolbar or the system.
    pub fn back(&mut self) -> BackResult {
        if self.prompt.is_some() {
            self.prompt = None;
            return BackResult::CancelledPrompt;
        }
        if let Some(sheet) = self.nav.sheet() {
            match sheet {
                SheetKind::Filters => {
                    let can_close = self
                        .editor
                        .as_mut()
                        .is_none_or(|editor| editor.filters.finish(self.text_input.as_mut()));
                    if can_close {
                        self.close_sheet();
                        BackResult::ClosedSheet
                    } else {
                        BackResult::Deferred
                    }
                }
                SheetKind::Export => {
                    let pending = self
                        .editor
                        .as_ref()
                        .and_then(|editor| editor.export.as_ref())
                        .is_some_and(ExportSheet::is_pending);
                    if pending {
                        BackResult::Ignored
                    } else {
                        self.close_sheet();
                        BackResult::ClosedSheet
                    }
                }
                _ => {
                    self.close_sheet();
                    BackResult::ClosedSheet
                }
            }
        } else if self.nav.document().is_some() {
            self.back_from_editor()
        } else {
            self.host.finish_activity();
            BackResult::Exit
        }
    }

    /// Answers the unsaved-changes prompt.
    pub fn answer_prompt(&mut self, choice: PromptChoice) {
        let Some(prompt) = self.prompt.take() else {
            return;
        };
        match choice {
            PromptChoice::Cancel => {}
            PromptChoice::DontSave => {
                if let Some(editor) = self.editor.as_mut()
                    && editor.document.is_large_mode()
                {
                    editor.document.revert_large_edits();
                }
                self.finish_action(prompt.action);
            }
            PromptChoice::Save => self.save_for_prompt(prompt.action),
        }
    }

    /// Opens a sheet from the toolbar, the overflow menu or the library.
    pub fn open_sheet(&mut self, sheet: SheetKind) {
        if self.nav.sheet() == Some(sheet) {
            return;
        }
        let needs_document = matches!(
            sheet,
            SheetKind::Format | SheetKind::Filters | SheetKind::Export
        );
        let Some(editor) = self.editor.as_mut() else {
            if needs_document {
                return;
            }
            self.nav.open_sheet(sheet);
            self.persist();
            return;
        };
        match sheet {
            SheetKind::Format => editor.format = FormatSheet::new(),
            SheetKind::Filters => editor.filters = FiltersSheet::new(),
            SheetKind::Export => editor.export = Some(ExportSheet::new(&editor.document)),
            SheetKind::Settings | SheetKind::About => {}
        }
        self.nav.open_sheet(sheet);
        self.persist();
    }

    /// Shares the current document through the platform.
    pub fn share_current_document(&mut self) {
        let path = self
            .editor
            .as_ref()
            .and_then(|editor| editor.document.file_path())
            .map(Path::to_path_buf);
        match path {
            Some(path) => self.host.share(&path),
            None => self.notice = Some(Notice::Error(ERROR_GENERIC)),
        }
    }

    /// Applies a toolbar response without drawing it.
    ///
    /// The mobile editor defers toolbar taps until an open edit commits, so
    /// the response already describes settled content.
    pub fn apply_editor_response(&mut self, response: &MobileEditorResponse) {
        if response.toolbar.back {
            let _ = self.back();
            return;
        }
        if response.toolbar.filter {
            self.open_sheet(SheetKind::Filters);
        }
        if response.toolbar.export {
            self.open_sheet(SheetKind::Export);
        }
        if let Some(item) = response.toolbar.overflow {
            match item {
                OverflowItem::Format => self.open_sheet(SheetKind::Format),
                OverflowItem::Settings => self.open_sheet(SheetKind::Settings),
                OverflowItem::About => self.open_sheet(SheetKind::About),
                OverflowItem::SaveAs => self.start_save_copy(),
            }
        }
        match response.large {
            Some(tabtool_ui::document::LargeUiAction::CancelSave) => self.cancel_large_save(),
            Some(tabtool_ui::document::LargeUiAction::ReloadStale) => self.reload_stale(),
            None => {}
        }
        if response.show_format_bar {
            self.open_sheet(SheetKind::Format);
        }
    }

    /// Applies a library response without drawing it.
    pub fn apply_library_response(&mut self, response: &LibraryResponse) {
        if let Some(path) = &response.open {
            self.request_open(path.clone());
        }
        if response.settings {
            self.open_sheet(SheetKind::Settings);
        }
        if response.about {
            self.open_sheet(SheetKind::About);
        }
    }

    /// Opens `path` in the editor and returns whether it loaded.
    pub fn open_document(&mut self, path: &Path) -> bool {
        let mut document = Document::new(self.settings.new_document_configuration());
        let opened = document.open_file(
            path,
            &StdStorage,
            None,
            self.prefers_chinese,
            self.settings.large_file_threshold_bytes(),
        );
        let ready = matches!(
            document.phase(),
            tabtool_document::DocumentPhase::Ready
                | tabtool_document::DocumentPhase::NeedsAttention(_)
        );
        if opened.is_err() || !ready {
            log::error!("could not open {}: {:?}", path.display(), opened);
            self.notice = Some(Notice::Error(ERROR_OPEN));
            return false;
        }
        self.editor = Some(Editor::new(document, path));
        self.nav.open_document(path.to_path_buf());
        self.prompt = None;
        self.pending = None;
        self.persist();
        true
    }

    fn metrics(&self, width: f32) -> MobileMetrics {
        let base = if width >= 600.0 {
            MobileMetrics::tablet()
        } else {
            MobileMetrics::phone()
        };
        metrics_for(
            base,
            &self.settings,
            tabtool_ui::mobile::Insets::from_array(self.host.safe_area()),
        )
    }

    fn appearance(&self, metrics: MobileMetrics) -> GridAppearance {
        GridAppearance {
            row_height: metrics.row_height,
            alternating_rows: self.settings.grid_alternating_rows,
            shows_row_numbers: self.settings.grid_shows_row_numbers,
        }
    }

    fn refresh_language(&mut self) -> Language {
        self.language = library::preferred_language(&self.settings, &self.languages);
        self.language
    }

    fn ensure_library(&mut self, metrics: MobileMetrics, language: Language) {
        let key = LibraryKey {
            language,
            metrics,
            configuration: self.settings.new_document_configuration(),
        };
        if key == self.library_key {
            return;
        }
        self.library = library::new_view(
            Rc::clone(&self.host),
            &self.settings,
            self.locale.clone(),
            self.clock,
            metrics,
            language,
        );
        self.library_key = key;
    }

    fn show_library(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        self.ensure_library(metrics, language);
        let response = self.library.show(ui, self.text_input.as_mut());
        self.apply_library_response(&response);
    }

    fn show_editor(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        let keyboard_height = self.host.keyboard_height();
        let appearance = self.appearance(metrics);
        let response = {
            let Some(editor) = self.editor.as_mut() else {
                return;
            };
            MobileEditor {
                document: &mut editor.document,
                state: &mut editor.grid,
                editor_state: &mut editor.state,
                text_input: self.text_input.as_mut(),
                clipboard: self.clipboard.as_mut(),
                metrics,
                language,
                title: &editor.title,
                locale: &self.locale,
                decimal_separator: &self.decimal_separator,
                appearance,
                keyboard_height,
                fold: None,
            }
            .show(ui)
        };
        self.apply_editor_response(&response);
    }

    fn show_sheets(&mut self, ui: &mut Ui, metrics: MobileMetrics, language: Language) {
        let Some(sheet) = self.nav.sheet() else {
            return;
        };
        let style = SheetStyle::new(metrics, language).keyboard_height(self.host.keyboard_height());
        match sheet {
            SheetKind::Settings => {
                let response = settings_sheet(ui, style, &mut self.settings);
                if response.changed {
                    self.host.save_settings(&self.settings);
                }
                if response.about {
                    self.open_sheet(SheetKind::About);
                } else if response.close_requested {
                    self.close_sheet();
                }
            }
            SheetKind::About => {
                let response = about_sheet(ui, style, tabtool_core::VERSION, LICENSE);
                if let Some(action) = response.action {
                    let url = match action {
                        AboutAction::Website => WEBSITE_URL,
                        AboutAction::Rate => tabtool_ui::mobile::RATE_URL,
                    };
                    self.host.open_url(url);
                }
                if response.close_requested {
                    self.close_sheet();
                }
            }
            SheetKind::Format => {
                let close = match self.editor.as_mut() {
                    Some(editor) => {
                        editor
                            .format
                            .show(
                                ui,
                                style,
                                &mut editor.document,
                                &StdStorage,
                                self.prefers_chinese,
                            )
                            .close_requested
                    }
                    None => false,
                };
                if close {
                    self.close_sheet();
                }
            }
            SheetKind::Filters => {
                let close = match self.editor.as_mut() {
                    Some(editor) => {
                        editor
                            .filters
                            .show(
                                ui,
                                style,
                                &mut editor.document,
                                self.text_input.as_mut(),
                                &self.locale,
                            )
                            .close_requested
                    }
                    None => false,
                };
                if close {
                    self.close_sheet();
                }
            }
            SheetKind::Export => {
                let close = match self.editor.as_mut() {
                    Some(editor) => match editor.export.as_mut() {
                        Some(export) => {
                            export
                                .show(ui, style, &editor.document, &self.files)
                                .close_requested
                        }
                        None => true,
                    },
                    None => true,
                };
                if close {
                    self.close_sheet();
                }
            }
        }
    }

    fn show_prompt(&mut self, ui: &mut Ui, language: Language) {
        let Some(prompt) = self.prompt.clone() else {
            return;
        };
        let title = tr(language, "Do you want to save the changes made to “%@”?")
            .replace("%@", &prompt.name);
        let message = tr(
            language,
            "Your changes will be lost if you don’t save them.",
        );
        let actions = [
            AlertAction::new(tr(language, "Save"), ButtonKind::Prominent),
            AlertAction::new(tr(language, "Don’t Save"), ButtonKind::Destructive),
            AlertAction::new(tr(language, "Cancel"), ButtonKind::Bordered),
        ];
        match modal(ui, "android-save-prompt", &title, message, &actions) {
            Some(0) => self.answer_prompt(PromptChoice::Save),
            Some(1) => self.answer_prompt(PromptChoice::DontSave),
            Some(_) => self.answer_prompt(PromptChoice::Cancel),
            None => {}
        }
    }

    fn draw_notice(&mut self, ui: &mut Ui, language: Language) {
        let Some(notice) = self.notice.clone() else {
            return;
        };
        let (kind, text) = match notice {
            Notice::Error(key) => (BannerKind::Error, tr(language, key).to_owned()),
            Notice::Message(message) => (BannerKind::Info, message),
        };
        let pressed = Panel::top("android-notice")
            .frame(Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| {
                banner(ui, kind, &text, "", Some(tr(language, "OK"))).action_clicked
            })
            .inner;
        if pressed {
            self.notice = None;
        }
    }

    fn close_sheet(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            match self.nav.sheet() {
                Some(SheetKind::Filters) => editor.filters = FiltersSheet::new(),
                Some(SheetKind::Format) => editor.format = FormatSheet::new(),
                Some(SheetKind::Export) => editor.export = None,
                _ => {}
            }
        }
        self.nav.close_sheet();
        self.persist();
    }

    fn back_from_editor(&mut self) -> BackResult {
        if self.text_input.is_active() {
            self.text_input.commit();
            self.pending = Some(Pending::Back);
            return BackResult::Deferred;
        }
        let Some(editor) = self.editor.as_mut() else {
            self.nav.back();
            return BackResult::ClosedDocument;
        };
        if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
            return BackResult::Ignored;
        }
        if !editor.document.is_dirty() {
            self.close_editor();
            return BackResult::ClosedDocument;
        }
        if lifecycle::close_needs_prompt(&editor.document) {
            self.prompt = Some(SavePrompt {
                action: CloseAction::CloseEditor,
                name: editor.title.clone(),
            });
            return BackResult::Prompt;
        }
        match editor.document.save(&StdStorage) {
            Ok(()) => {
                self.close_editor();
                BackResult::ClosedDocument
            }
            Err(error) => {
                log::error!("could not save before closing: {error:?}");
                self.prompt = Some(SavePrompt {
                    action: CloseAction::CloseEditor,
                    name: editor.title.clone(),
                });
                BackResult::Prompt
            }
        }
    }

    fn close_editor(&mut self) {
        self.editor = None;
        self.pending = None;
        self.background_wait = None;
        self.prompt = None;
        while self.nav.close_sheet() {}
        self.nav.back();
        self.persist();
    }

    fn request_open(&mut self, path: PathBuf) {
        if let Some(editor) = self.editor.as_mut() {
            if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
                self.notice = Some(Notice::Error(ERROR_GENERIC));
                return;
            }
            if editor.document.is_dirty() {
                if lifecycle::close_needs_prompt(&editor.document) {
                    self.prompt = Some(SavePrompt {
                        action: CloseAction::Open(path),
                        name: editor.title.clone(),
                    });
                    return;
                }
                if let Err(error) = editor.document.save(&StdStorage) {
                    log::error!("could not save before opening another document: {error:?}");
                    self.notice = Some(Notice::Error(ERROR_SAVE));
                    return;
                }
            }
        }
        self.open_document(&path);
    }

    fn save_for_prompt(&mut self, action: CloseAction) {
        let mut saved = false;
        let mut failed = false;
        if let Some(editor) = self.editor.as_mut() {
            if editor.document.is_large_mode() {
                match editor.document.start_large_save(None) {
                    Ok(job) => {
                        drop(job);
                        editor.pending_after_save = Some(action.clone());
                    }
                    Err(error) => {
                        log::error!("could not start the large save: {error:?}");
                        failed = true;
                    }
                }
            } else {
                match editor.document.save(&StdStorage) {
                    Ok(()) => saved = true,
                    Err(error) => {
                        log::error!("could not save: {error:?}");
                        failed = true;
                    }
                }
            }
        } else {
            saved = true;
        }
        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        if saved {
            self.finish_action(action);
        }
    }

    fn finish_action(&mut self, action: CloseAction) {
        match action {
            CloseAction::CloseEditor => self.close_editor(),
            CloseAction::Open(path) => {
                self.open_document(&path);
            }
        }
    }

    fn start_save_copy(&mut self) {
        let files = library::AppFiles::new(Rc::clone(&self.host));
        let language = self.language;
        let mut result = None;
        if let Some(editor) = self.editor.as_mut() {
            if !matches!(editor.document.large_status(), LargeStatus::Ready) {
                return;
            }
            editor.save_copy.start(&editor.document, &files);
            result = editor.save_copy.take_result();
        }
        self.apply_copy_result(result, language);
    }

    fn apply_copy_result(&mut self, result: Option<CopyResult>, language: Language) {
        match result {
            Some(CopyResult::Saved(name)) => {
                self.notice = Some(Notice::Message(saved_copy_message(language, &name)));
            }
            Some(CopyResult::Failed) => self.notice = Some(Notice::Error(ERROR_SAVE)),
            Some(CopyResult::Cancelled) | None => {}
        }
    }

    fn cancel_large_save(&mut self) {
        self.background_wait = None;
        if let Some(editor) = self.editor.as_mut() {
            editor.document.cancel_large_save();
            let _ = editor.document.wait_for_large_save();
            editor.pending_after_save = None;
        }
    }

    fn reload_stale(&mut self) {
        let result = self
            .editor
            .as_mut()
            .map(|editor| editor.document.reload_large(&StdStorage));
        if matches!(result, Some(Err(_))) {
            self.notice = Some(Notice::Error(ERROR_OPEN));
        }
    }

    /// Writes a pending background save once no native edit owns the content.
    fn flush_background(&mut self, now: Instant) {
        if !self.background_pending {
            return;
        }
        if self.text_input.is_active()
            || self
                .editor
                .as_ref()
                .is_some_and(|editor| editor.state.editing.is_some())
        {
            return;
        }
        self.background_pending = false;
        let mut failed = false;
        if let Some(editor) = self.editor.as_mut() {
            match lifecycle::save_for_background(&mut editor.document, &StdStorage) {
                Ok(true) => {}
                Ok(false) => {
                    if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
                        self.background_wait = Some(now + BACKGROUND_LARGE_WAIT);
                    }
                }
                Err(error) => {
                    log::error!("background save failed: {error:?}");
                    failed = true;
                }
            }
        }
        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        self.host.background_save_finished();
    }

    fn poll_editor(&mut self, now: Instant) {
        let background_wait = self.background_wait;
        let mut failed = false;
        let mut copy_result = None;
        let mut after_save = None;
        let files = library::AppFiles::new(Rc::clone(&self.host));
        let language = self.language;
        let prompt_open = self.prompt.is_some();

        if let Some(editor) = self.editor.as_mut() {
            if !prompt_open
                && let Some(error) = lifecycle::tick_autosave(
                    &mut editor.autosave,
                    now,
                    &mut editor.document,
                    &StdStorage,
                )
                && !matches!(error, DocumentError::Csv(_))
            {
                failed = true;
            }

            editor.save_copy.poll(&files);
            copy_result = editor.save_copy.take_result();

            if matches!(editor.document.large_status(), LargeStatus::Saving(_)) {
                let finished = editor
                    .document
                    .large_save_progress()
                    .is_some_and(|progress| progress >= 1.0)
                    || editor.document.large_save_finished();
                if finished {
                    match editor.document.wait_for_large_save() {
                        Ok(()) => after_save = editor.pending_after_save.take(),
                        Err(error) => {
                            log::error!("large save failed: {error:?}");
                            editor.pending_after_save = None;
                            failed = true;
                        }
                    }
                }
            } else if let Some(action) = editor.pending_after_save.take() {
                // A save that settled between frames still needs its join so
                // the mapping is adopted before the app opens or closes.
                match editor.document.wait_for_large_save() {
                    Ok(()) => after_save = Some(action),
                    Err(error) => {
                        log::error!("large save failed: {error:?}");
                        failed = true;
                    }
                }
            }
        }

        if let Some(deadline) = background_wait
            && now >= deadline
        {
            self.background_wait = None;
        }
        if failed {
            self.notice = Some(Notice::Error(ERROR_SAVE));
        }
        self.apply_copy_result(copy_result, language);
        if let Some(action) = after_save {
            self.finish_action(action);
        }
    }

    fn poll_pending(&mut self) {
        if self.pending != Some(Pending::Back) || self.text_input.is_active() {
            return;
        }
        if self
            .editor
            .as_ref()
            .is_some_and(|editor| editor.state.editing.is_some())
        {
            return;
        }
        self.pending = None;
        let _ = self.back();
    }

    fn poll_platform_error(&mut self) {
        if let Some(error) = self.host.take_error() {
            self.notice = Some(Notice::Error(error.message_key));
        }
    }

    fn poll_intents(&mut self) {
        while let Some(event) = self.host.take_intent() {
            match event {
                intents::IntentEvent::Staged(staged) => {
                    match intents::adopt(&self.host.documents_dir(), &staged) {
                        Ok(path) => self.request_open(path),
                        Err(error) => {
                            log::error!("could not adopt the intent document: {error}");
                            self.notice = Some(Notice::Error(ERROR_OPEN));
                        }
                    }
                }
                intents::IntentEvent::Failed(detail) => {
                    log::error!("the intent copy failed: {detail}");
                    self.notice = Some(Notice::Error(ERROR_OPEN));
                }
            }
        }
    }

    fn sync_nav_notice(&mut self) {
        if let Some(key) = self.nav.take_notice() {
            self.notice = Some(Notice::Error(key));
        }
    }

    fn persist(&self) {
        if let Err(error) =
            lifecycle::save_state(self.host.state_path().as_deref(), &self.nav.snapshot())
        {
            log::error!("could not persist the session: {error}");
        }
    }

    fn restore(&mut self, snapshot: &NavSnapshot) {
        let mut missing = false;
        if let Some(path) = &snapshot.document {
            let directory = self.host.documents_dir();
            if lifecycle::restorable(&directory, path) {
                if !self.open_document(path) {
                    missing = true;
                }
            } else {
                missing = true;
            }
        }
        if missing {
            self.nav.document_missing();
            self.sync_nav_notice();
            self.persist();
        }
        if let Some(sheet) = snapshot.sheet
            && (self.editor.is_some() || lifecycle::library_sheet(sheet))
        {
            self.open_sheet(sheet);
        }
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

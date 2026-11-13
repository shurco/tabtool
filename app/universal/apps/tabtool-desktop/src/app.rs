//! The eframe application shell: viewports, menu dispatch and quit policy.

mod dispatch;
mod files;
mod quit;
mod recent;
mod viewports;
mod welcome;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use egui::{Context, Ui};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, Settings, settings_path};

use crate::documents::DocumentWindow;
use crate::files::FileWorkflow;
use crate::platform::{
    SharedDialogs, SharedStorage, interface_language, standard_dialogs, standard_storage,
};

pub use quit::{QuitDecision, QuitResponse, quit_decision};

/// The desktop application.
pub struct DesktopApp {
    /// One window per document, in creation order.
    pub documents: Vec<DocumentWindow>,
    /// The persisted settings.
    pub settings: Settings,
    /// Whether the About window is open.
    pub about: bool,
    /// Whether the Settings window is open.
    pub settings_window: bool,
    /// The platform services.
    pub platform: DesktopPlatform,
    /// File workflows and the recent-document list.
    pub files: FileWorkflow,
    storage: SharedStorage,
    dialogs: SharedDialogs,
    next_window: u64,
    untitled_count: usize,
    language: Language,
    recent_picker: bool,
    root_visible: Option<bool>,
    about_closed: Arc<AtomicBool>,
    settings_closed: Arc<AtomicBool>,
    quit: quit::QuitState,
}

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

    fn build(
        settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
        dialogs: SharedDialogs,
        settings_path: Option<PathBuf>,
    ) -> Self {
        let language = interface_language(&settings, &platform);
        let mut files = FileWorkflow::new(
            Arc::clone(&dialogs),
            Arc::clone(&storage),
            settings.clone(),
            settings_path,
        );
        files.set_language(language);
        Self {
            documents: Vec::new(),
            settings,
            about: false,
            settings_window: false,
            platform,
            files,
            storage,
            dialogs,
            next_window: 0,
            untitled_count: 0,
            language,
            recent_picker: false,
            root_visible: None,
            about_closed: Arc::new(AtomicBool::new(false)),
            settings_closed: Arc::new(AtomicBool::new(false)),
            quit: quit::QuitState::default(),
        }
    }

    /// The open document windows.
    pub fn documents(&self) -> &[DocumentWindow] {
        &self.documents
    }

    /// The resolved interface language.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Creates an empty untitled document in the New Document format.
    pub fn new_document(&mut self) {
        let document = self.files.new_document();
        self.untitled_count += 1;
        let name = untitled_name(self.untitled_count);
        self.push_document(document, name);
    }

    /// Adds an existing document window.
    pub fn add_document(&mut self, document: Document) {
        let name = if document.file_path().is_none() {
            self.untitled_count += 1;
            untitled_name(self.untitled_count)
        } else {
            String::new()
        };
        self.push_document(document, name);
    }

    /// Opens the Settings window.
    pub fn open_settings(&mut self) {
        self.settings_window = true;
    }

    /// Opens the About window.
    pub fn open_about(&mut self) {
        self.about = true;
    }

    /// Advances timers, dispatches window requests and registers viewports.
    pub fn tick(&mut self, ctx: &Context) {
        self.sync_language();
        self.files.set_settings(self.settings.clone());
        self.files.set_language(self.language);
        self.handle_root_close(ctx);
        self.close_finished_aux_windows();

        let now = Instant::now();
        let has_recent = self.files.has_recent();
        for window in &self.documents {
            window.set_chrome(self.language, self.settings.show_format_bar, has_recent);
            window.set_settings(self.settings.clone());
        }

        let mut actions = Vec::new();
        for window in &self.documents {
            actions.extend(window.tick(now, ctx));
        }

        let mut opened_paths = Vec::new();
        let mut dropped = Vec::new();
        for window in &self.documents {
            opened_paths.extend(window.take_opened_paths());
            dropped.extend(window.take_dropped_files());
        }
        for path in opened_paths {
            self.files.remember(&path);
        }
        self.sync_recent();
        if !dropped.is_empty() {
            self.open_dropped(&dropped);
        }

        for action in actions {
            self.perform(action, ctx);
        }

        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.open_dropped(&dropped);
        }

        self.poll_quit(ctx);
        self.remove_closed(ctx);
        self.register_viewports(ctx);
        self.sync_root_visibility(ctx);
        self.schedule_repaint(ctx, now);
    }

    /// Renders the root viewport: the empty state, recent chooser and quit alerts.
    pub fn show(&mut self, ui: &mut Ui) {
        if self.documents.is_empty() {
            self.render_welcome(ui);
        }
        self.render_recent_documents(ui);
        self.render_quit_alerts(ui);
    }
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

fn untitled_name(count: usize) -> String {
    if count == 1 {
        "Untitled".to_owned()
    } else {
        format!("Untitled {count}")
    }
}

//! The eframe application shell: viewports, menu dispatch and quit policy.

mod constructors;
mod dispatch;
mod files;
mod preferences;
mod quit;
mod recent;
mod viewports;
mod welcome;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use egui::{Context, Ui};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_settings::Settings;

use crate::about_window::AboutAction;
use crate::documents::DocumentWindow;
use crate::files::FileWorkflow;
use crate::platform::{SharedDialogs, SharedStorage};
use crate::settings_window::SettingsWindow;

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
    settings_path: Option<PathBuf>,
    settings_window_state: Option<SettingsWindow>,
    settings_dirty: bool,
    next_window: u64,
    untitled_count: usize,
    language: Language,
    /// Whether the one-time system font coverage check has run.
    fonts_checked: bool,
    /// The system languages, cached because `sys_locale` is process-wide and
    /// must not be queried every frame.
    system_languages: Vec<String>,
    prefers_chinese: bool,
    recent_picker: bool,
    root_visible: Option<bool>,
    about_closed: Arc<AtomicBool>,
    about_action: Arc<Mutex<Option<AboutAction>>>,
    quit: quit::QuitState,
}

impl DesktopApp {
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
        if self.settings_window {
            return;
        }
        self.settings_window_state = Some(SettingsWindow::new(self.settings.clone()));
        self.settings_window = true;
    }

    /// Opens the About window.
    pub fn open_about(&mut self) {
        self.about = true;
    }

    /// Advances timers, dispatches window requests and registers viewports.
    pub fn tick(&mut self, ctx: &Context) {
        self.tick_at(Instant::now(), ctx);
    }

    /// The scheduler pass with an injected clock, for deterministic tests.
    pub fn tick_at(&mut self, now: Instant, ctx: &Context) {
        self.sync_settings_window();
        self.sync_language();
        self.files.set_settings(self.settings.clone());
        self.sync_recent();
        self.handle_root_close(ctx);
        self.close_finished_aux_windows();
        self.take_about_action();

        let has_recent = self.files.has_recent();
        let quit_modal = self.quit.prompt
            || self.quit.busy
            || self.quit.issue.is_some()
            || !self.quit.saving.is_empty();
        for window in &self.documents {
            window.set_chrome(self.language, self.settings.show_format_bar, has_recent);
            window.set_settings(self.settings.clone());
            window.set_modal_blocked(quit_modal);
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
        self.persist_settings();
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
        // The first frame has built egui's font set, which the coverage check
        // needs. A missing script only warns; the app keeps rendering.
        if !self.fonts_checked {
            self.fonts_checked = true;
            tabtool_ui::fonts::warn_missing_scripts(ctx);
        }
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

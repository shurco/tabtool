//! The eframe application shell: viewports, menu dispatch and quit policy.

mod quit;
mod viewports;
mod welcome;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use egui::{Context, Ui, ViewportCommand, ViewportId};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_platform::{DesktopPlatform, OpenUrl, Settings};

use crate::documents::{DocumentWindow, WindowServices};
use crate::menu::MenuAction;
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
    storage: SharedStorage,
    dialogs: SharedDialogs,
    next_window: u64,
    untitled_count: usize,
    language: Language,
    pending_actions: Vec<MenuAction>,
    root_visible: Option<bool>,
    about_closed: Arc<AtomicBool>,
    settings_closed: Arc<AtomicBool>,
    quit: quit::QuitState,
}

impl DesktopApp {
    /// Creates an empty app with the filesystem storage.
    pub fn new(settings: Settings, platform: DesktopPlatform) -> Self {
        Self::with_storage(settings, platform, standard_storage())
    }

    /// Creates an empty app with an injected storage.
    pub fn with_storage(
        settings: Settings,
        platform: DesktopPlatform,
        storage: SharedStorage,
    ) -> Self {
        Self::with_dialogs(settings, platform, storage, standard_dialogs())
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
        let language = interface_language(&settings, &platform);
        Self {
            documents: Vec::new(),
            settings,
            about: false,
            settings_window: false,
            platform,
            storage,
            dialogs,
            next_window: 0,
            untitled_count: 0,
            language,
            pending_actions: Vec::new(),
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

    /// The actions the shell still has to hand to a later view.
    pub fn pending_actions(&self) -> &[MenuAction] {
        &self.pending_actions
    }

    /// The resolved interface language.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Creates an empty untitled document.
    pub fn new_document(&mut self) {
        let configuration = self.settings.new_document_configuration();
        let mut document = Document::new(configuration.clone());
        document
            .new_document(configuration)
            .expect("a new untitled document is never busy");
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
        self.handle_root_close(ctx);
        self.close_finished_aux_windows();

        let now = Instant::now();
        let has_recent = !self.settings.recent_documents.is_empty();
        for window in &self.documents {
            window.set_chrome(self.language, self.settings.show_format_bar, has_recent);
        }

        let mut actions = Vec::new();
        for window in &self.documents {
            actions.extend(window.tick(now, ctx));
        }
        for action in actions {
            self.perform(action, ctx);
        }

        self.poll_quit(ctx);
        self.remove_closed(ctx);
        self.register_viewports(ctx);
        self.sync_root_visibility(ctx);
        self.schedule_repaint(ctx, now);
    }

    /// Renders the root viewport: the empty state and the quit alerts.
    pub fn show(&mut self, ui: &mut Ui) {
        if self.documents.is_empty() {
            self.render_welcome(ui);
        }
        self.render_quit_alerts(ui);
    }

    fn push_document(&mut self, document: Document, untitled_name: String) {
        let serial = self.next_window;
        self.next_window += 1;
        let viewport_id = ViewportId::from_hash_of(("tabtool-document", serial));
        let window = DocumentWindow::new(
            viewport_id,
            document,
            untitled_name,
            self.language,
            self.settings.show_format_bar,
            WindowServices {
                platform: self.platform,
                dialogs: Arc::clone(&self.dialogs),
                storage: Arc::clone(&self.storage),
            },
        );
        self.documents.push(window);
    }

    pub(crate) fn perform(&mut self, action: MenuAction, ctx: &Context) {
        match action {
            MenuAction::About => self.about = true,
            MenuAction::Settings => self.settings_window = true,
            MenuAction::Quit => self.request_quit(ctx),
            MenuAction::New => self.new_document(),
            MenuAction::Open | MenuAction::OpenRecent => self.pending_actions.push(action),
            MenuAction::ToggleFormatBar => {
                self.settings.show_format_bar = !self.settings.show_format_bar;
            }
            MenuAction::Website => self.platform.open("https://tabtool.shurco.app"),
            MenuAction::License => self
                .platform
                .open("https://github.com/shurco/tabtool/blob/main/LICENSE"),
            _ => {}
        }
    }

    fn sync_language(&mut self) {
        self.language = interface_language(&self.settings, &self.platform);
    }

    fn handle_root_close(&mut self, ctx: &Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.quit.force_close {
            // The user already chose Don't Save or every document was saved.
            // Let the close proceed even though documents may still report
            // unsaved edits.
            return;
        }
        match quit_decision(&self.documents) {
            QuitDecision::Quit => {}
            QuitDecision::Ask => {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.quit.prompt = true;
            }
            QuitDecision::Busy => {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.quit.busy = true;
            }
        }
    }

    fn close_finished_aux_windows(&mut self) {
        if self.about_closed.swap(false, Ordering::SeqCst) {
            self.about = false;
        }
        if self.settings_closed.swap(false, Ordering::SeqCst) {
            self.settings_window = false;
        }
    }

    fn remove_closed(&mut self, ctx: &Context) {
        let mut closed = Vec::new();
        self.documents.retain(|window| {
            if window.should_close() {
                closed.push(window.viewport_id());
                false
            } else {
                true
            }
        });
        for viewport_id in closed {
            ctx.send_viewport_cmd_to(viewport_id, ViewportCommand::Close);
        }
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

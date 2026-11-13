//! The Settings window: a deferred viewport around the reusable settings form.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use egui::{Context, Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId};
use tabtool_i18n::{Language, tr};
use tabtool_platform::Settings;
use tabtool_ui::settings::{SettingsForm, show};

/// The deferred Settings viewport and its shared form state.
#[derive(Clone)]
pub struct SettingsWindow {
    form: Arc<Mutex<SettingsForm>>,
    changed: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
}

impl SettingsWindow {
    /// Creates the window for `settings`.
    pub fn new(settings: Settings) -> Self {
        Self {
            form: Arc::new(Mutex::new(SettingsForm::new(settings))),
            changed: Arc::new(AtomicBool::new(false)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Takes the settings the form last changed, or `None` when it is clean.
    ///
    /// The app merges only form changes; settings changed elsewhere, such as
    /// View > Show Format Bar, flow the other way through [`Self::set_settings`].
    pub fn take_changed(&self) -> Option<Settings> {
        if !self.changed.swap(false, Ordering::SeqCst) {
            return None;
        }
        Some(self.lock().settings.clone())
    }

    /// Mirrors settings changed outside the window, e.g. View > Show Format Bar.
    pub fn set_settings(&self, settings: Settings) {
        let mut form = self.lock();
        if form.settings != settings {
            form.settings = settings;
        }
    }

    /// Whether the native close request was seen.
    pub fn take_closed(&self) -> bool {
        self.closed.swap(false, Ordering::SeqCst)
    }

    /// Registers the deferred viewport for this frame.
    pub fn show_viewport(&self, ctx: &Context, language: Language) {
        let window = self.clone();
        let builder = ViewportBuilder::default()
            .with_title(tr(language, "Settings"))
            .with_inner_size(window.preferred_size())
            .with_min_inner_size([420.0, 140.0]);
        ctx.show_viewport_deferred(viewport_id(), builder, move |ui, _class| {
            window.show(ui, language);
        });
    }

    /// Draws one frame; returns whether a preference changed.
    pub fn show(&self, ui: &mut Ui, system_language: Language) -> bool {
        if ui.ctx().input(|input| input.viewport().close_requested()) {
            self.closed.store(true, Ordering::SeqCst);
        }
        let theme = tabtool_ui::theme::current(ui.ctx());
        let mut form = self.lock();
        let language = form.effective_language(system_language);
        let response = egui::Frame::new()
            .fill(theme.colors.window_background)
            .inner_margin(theme.metrics.window_padding)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                show(ui, &mut form, language)
            })
            .inner;
        if response.tab_changed {
            let [width, height] = form.preferred_size();
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(width, height)));
        }
        if response.changed {
            self.changed.store(true, Ordering::SeqCst);
        }
        response.changed
    }

    fn preferred_size(&self) -> Vec2 {
        let [width, height] = self.lock().preferred_size();
        Vec2::new(width, height)
    }

    fn lock(&self) -> MutexGuard<'_, SettingsForm> {
        self.form
            .lock()
            .expect("the settings form lock is not poisoned")
    }
}

/// The Settings viewport identity.
pub fn viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-settings")
}

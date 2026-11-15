//! The Settings window: a deferred viewport around the reusable settings form.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use egui::{Context, Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId};
use tabtool_i18n::{Language, tr};
use tabtool_settings::Settings;
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
        let close_requested = ui.ctx().input(|input| input.viewport().close_requested());
        if close_requested {
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
        wake_root(
            ui.ctx(),
            response.changed,
            response.tab_changed,
            close_requested,
        );
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

/// Wakes the root pass when the Settings window has work for it.
///
/// The root merges the form and persists settings; a child repaint alone does
/// not run it, so a change, a tab switch (which resizes the window) and a close
/// each need a root pass.
fn wake_root(ctx: &Context, changed: bool, tab_changed: bool, close_requested: bool) {
    if changed || tab_changed || close_requested {
        ctx.request_repaint_of(ViewportId::ROOT);
    }
}

/// The Settings viewport identity.
pub fn viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-settings")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use egui::{Context, ViewportId};

    use super::wake_root;

    fn requested(changed: bool, tab_changed: bool, close_requested: bool) -> bool {
        let ctx = Context::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&seen);
        ctx.set_repaint_observer(move |info| {
            record
                .lock()
                .expect("the observer lock is not poisoned")
                .push(info.viewport_id);
        });
        wake_root(&ctx, changed, tab_changed, close_requested);
        seen.lock()
            .expect("the observer lock is not poisoned")
            .contains(&ViewportId::ROOT)
    }

    #[test]
    fn every_root_consumer_wakes_the_logic_pass() {
        assert!(requested(true, false, false), "a change must wake the root");
        assert!(
            requested(false, true, false),
            "a tab switch must wake the root"
        );
        assert!(requested(false, false, true), "a close must wake the root");
        assert!(!requested(false, false, false));
    }
}

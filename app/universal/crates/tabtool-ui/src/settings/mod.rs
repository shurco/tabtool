//! The Settings form: four tabs of persisted preferences.
//!
//! The form owns a [`Settings`] copy and reports every change to the caller.
//! The desktop window persists the copy and applies it to open documents; the
//! form itself never touches the filesystem, a document or the platform.

pub(crate) mod options;
mod tabs;

use egui::Ui;
use tabtool_i18n::{Language, tr};
use tabtool_settings::Settings;

use crate::widgets::tab_bar;

use tabs::{general, grid, large_files, new_document};

/// The selected Settings tab.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingsTab {
    /// Language and the format bar.
    #[default]
    General,
    /// Defaults for a new document.
    NewDocument,
    /// Grid appearance.
    Grid,
    /// The large-file threshold.
    LargeFiles,
}

impl SettingsTab {
    /// The tabs in display order.
    pub const ALL: [Self; 4] = [
        Self::General,
        Self::NewDocument,
        Self::Grid,
        Self::LargeFiles,
    ];

    /// The tab's catalog key.
    pub fn key(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::NewDocument => "New Document",
            Self::Grid => "Grid",
            Self::LargeFiles => "Large Files",
        }
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|tab| *tab == self)
            .expect("every tab is listed")
    }

    fn at(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }
}

/// The mutable state of the Settings window.
#[derive(Clone, Debug)]
pub struct SettingsForm {
    /// The preferences the controls edit.
    pub settings: Settings,
    /// The selected tab.
    pub tab: SettingsTab,
}

impl SettingsForm {
    /// A form editing `settings`, on the General tab.
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            tab: SettingsTab::General,
        }
    }

    /// The language this form renders in.
    ///
    /// An explicit preference wins; [`Language::System`] follows `system`.
    pub fn effective_language(&self, system: Language) -> Language {
        match self.settings.app_language {
            Language::System => system,
            language => language,
        }
    }

    /// The window size that fits the selected tab.
    pub fn preferred_size(&self) -> [f32; 2] {
        let height = match self.tab {
            SettingsTab::General => 180.0,
            SettingsTab::NewDocument => 330.0,
            SettingsTab::Grid => 230.0,
            SettingsTab::LargeFiles => 160.0,
        };
        [520.0, height]
    }
}

/// What the form asked its caller to do.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsResponse {
    /// At least one preference changed.
    pub changed: bool,
    /// The window should resize to [`SettingsForm::preferred_size`].
    pub tab_changed: bool,
}

/// Draws the Settings form.
pub fn show(ui: &mut Ui, form: &mut SettingsForm, language: Language) -> SettingsResponse {
    let mut response = SettingsResponse::default();
    let mut selected = form.tab.index();
    let labels: Vec<&str> = SettingsTab::ALL
        .iter()
        .map(|tab| tr(language, tab.key()))
        .collect();
    if tab_bar(ui, &labels, &mut selected).changed() && selected != form.tab.index() {
        form.tab = SettingsTab::at(selected);
        response.tab_changed = true;
    }

    ui.add_space(10.0);
    let mut changed = false;
    match form.tab {
        SettingsTab::General => general(ui, &mut form.settings, language, &mut changed),
        SettingsTab::NewDocument => {
            new_document(ui, &mut form.settings, language, &mut changed);
        }
        SettingsTab::Grid => grid(ui, &mut form.settings, language, &mut changed),
        SettingsTab::LargeFiles => large_files(ui, &mut form.settings, language, &mut changed),
    }
    response.changed = changed;
    response
}

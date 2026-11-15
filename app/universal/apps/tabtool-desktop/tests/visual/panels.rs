//! Settings, About, Export and root-window snapshot states.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use egui::{Align, Layout, Vec2};
use tabtool_desktop::app::DesktopApp;
use tabtool_desktop::platform::SharedStorage;
use tabtool_desktop::settings_window::SettingsWindow;
use tabtool_i18n::Language;
use tabtool_platform::DesktopPlatform;
use tabtool_settings::Settings;
use tabtool_ui::UiHarness;
use tabtool_ui::export::export_panel;
use tabtool_ui::harness::kittest::Queryable;

use super::fixture::{csv_configuration, paint_background};
use crate::support::MemoryStorage;

/// The Settings window frame on one tab.
pub fn settings_harness(theme: tabtool_ui::Theme, size: [f32; 2], tab: &str) -> UiHarness<'static> {
    let window = Rc::new(RefCell::new(SettingsWindow::new(Settings::default())));
    let mut harness = UiHarness::new(theme, size, move |ui| {
        paint_background(ui, theme);
        let width = 520.0_f32.min(ui.available_width());
        ui.allocate_ui_with_layout(
            Vec2::new(width, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                window.borrow().show(ui, Language::En);
            },
        );
    });
    harness.run();
    if tab != "General" {
        harness.get_by_label(tab).click();
        // Selecting a tab asks the window to resize. Restore the requested
        // baseline size so the 1200×800 and 640×400 images stay distinct.
        harness.run();
        harness.set_size(Vec2::new(size[0], size[1]));
        harness.run();
    }
    harness.set_size(Vec2::new(size[0], size[1]));
    harness.remove_cursor();
    harness.run();
    harness
}

/// The About window content.
pub fn about_harness(theme: tabtool_ui::Theme, size: [f32; 2]) -> UiHarness<'static> {
    let version = env!("CARGO_PKG_VERSION");
    let license = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../LICENSE"));
    let mut harness = UiHarness::new(theme, size, move |ui| {
        paint_background(ui, theme);
        let width = 420.0_f32.min(ui.available_width());
        ui.allocate_ui_with_layout(
            Vec2::new(width, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                let _ = tabtool_ui::about::show(ui, Language::En, version, license);
            },
        );
    });
    harness.run();
    harness.remove_cursor();
    harness.run();
    harness
}

/// The export options modal over the document surface.
pub fn export_harness(theme: tabtool_ui::Theme, size: [f32; 2]) -> UiHarness<'static> {
    let mut configuration = csv_configuration();
    let mut harness = UiHarness::new(theme, size, move |ui| {
        let _ = export_panel(ui, "visual-export", &mut configuration, Language::En);
    });
    harness.run();
    harness.remove_cursor();
    harness.run();
    harness
}

/// The root window's empty state.
pub fn welcome_harness(theme: tabtool_ui::Theme, size: [f32; 2]) -> UiHarness<'static> {
    let storage: SharedStorage = Arc::new(MemoryStorage::new(b""));
    let app = Rc::new(RefCell::new(DesktopApp::with_storage(
        Settings::default(),
        DesktopPlatform,
        storage,
    )));
    let mut harness = UiHarness::new(theme, size, move |ui| {
        paint_background(ui, theme);
        let mut app = app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    });
    harness.run();
    harness.remove_cursor();
    harness.run();
    harness
}

//! The About window wiring: version, links and the bundled license.

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tabtool_desktop::about_window;
use tabtool_desktop::app::DesktopApp;
use tabtool_desktop::platform::SharedStorage;
use tabtool_platform::DesktopPlatform;
use tabtool_settings::Settings;
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::{Theme, UiHarness};

use support::MemoryStorage;

fn app() -> Rc<RefCell<DesktopApp>> {
    let storage: SharedStorage = Arc::new(MemoryStorage::new(b""));
    Rc::new(RefCell::new(DesktopApp::with_storage(
        Settings::default(),
        DesktopPlatform,
        storage,
    )))
}

fn render(handle: &Rc<RefCell<DesktopApp>>) -> UiHarness<'static> {
    let app = Rc::clone(handle);
    UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut app = app.borrow_mut();
        app.tick(ui.ctx());
        app.show(ui);
    })
}

#[test]
fn the_license_is_the_bundled_mit_text() {
    let license = about_window::license_text();
    assert!(license.starts_with("The MIT License (MIT)"));
    assert!(license.contains("Copyright (c) 2026 Dmitry Shurco"));
    assert!(license.contains("THE SOFTWARE IS PROVIDED \"AS IS\""));
}

#[test]
fn the_about_links_point_to_the_site_and_the_review_page() {
    assert_eq!(about_window::WEBSITE_URL, "https://tabtool.shurco.app");
    assert!(about_window::REVIEW_URL.starts_with("https://apps.apple.com/"));
    assert!(about_window::REVIEW_URL.contains("action=write-review"));
}

#[test]
fn the_about_window_shows_the_identity_and_license_notice() {
    let handle = app();
    handle.borrow_mut().open_about();
    let mut harness = render(&handle);
    harness.run();

    assert!(
        harness.query_all_by_label("TabTool").next().is_some(),
        "the About window names the app"
    );
    assert!(
        harness
            .query_by_label(&format!("Version {}", tabtool_core::VERSION))
            .is_some()
    );
    assert!(harness.query_by_label("tabtool.shurco.app").is_some());
    assert!(harness.query_by_label("Rate TabTool").is_some());
    assert!(harness.query_by_label("MIT License").is_some());
    assert!(
        harness
            .query_by_label("Free and open source. Copyright © 2026 Dmitry Shurco.")
            .is_some()
    );
}

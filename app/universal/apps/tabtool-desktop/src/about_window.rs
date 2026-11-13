//! The About window: the app's identity, links and bundled license.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use egui::{Context, ViewportBuilder, ViewportId};
use tabtool_i18n::Language;

pub use tabtool_ui::about::AboutAction;

/// The project website.
pub const WEBSITE_URL: &str = "https://tabtool.shurco.app";

/// The App Store review page.
pub const REVIEW_URL: &str = "https://apps.apple.com/app/id6827919069?action=write-review";

/// The desktop crate's version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The MIT license bundled with the app.
pub fn license_text() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../LICENSE"))
}

/// Registers the deferred About viewport for this frame.
pub fn show_viewport(
    ctx: &Context,
    language: Language,
    closed: Arc<AtomicBool>,
    action: Arc<Mutex<Option<AboutAction>>>,
) {
    let builder = ViewportBuilder::default()
        .with_title(tabtool_i18n::tr(language, "About TabTool"))
        .with_inner_size([420.0, 430.0])
        .with_min_inner_size([320.0, 240.0]);
    ctx.show_viewport_deferred(viewport_id(), builder, move |ui, _class| {
        if ui.ctx().input(|input| input.viewport().close_requested()) {
            closed.store(true, Ordering::SeqCst);
        }
        if let Some(chosen) = tabtool_ui::about::show(ui, language, version(), license_text()) {
            *action
                .lock()
                .expect("the about action lock is not poisoned") = Some(chosen);
        }
    });
}

/// The About viewport identity.
pub fn viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-about")
}

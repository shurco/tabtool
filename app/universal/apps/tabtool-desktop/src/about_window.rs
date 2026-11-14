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
        let close_requested = ui.ctx().input(|input| input.viewport().close_requested());
        if close_requested {
            closed.store(true, Ordering::SeqCst);
        }
        let chosen = tabtool_ui::about::show(ui, language, version(), license_text());
        if let Some(chosen) = chosen {
            *action
                .lock()
                .expect("the about action lock is not poisoned") = Some(chosen);
        }
        wake_root(ui.ctx(), chosen.is_some(), close_requested);
    });
}

/// Wakes the root pass when the About window has work for it.
///
/// The root opens the link and clears the closed flag; a child repaint alone
/// does not run it.
fn wake_root(ctx: &Context, has_action: bool, close_requested: bool) {
    if has_action || close_requested {
        ctx.request_repaint_of(ViewportId::ROOT);
    }
}

/// The About viewport identity.
pub fn viewport_id() -> ViewportId {
    ViewportId::from_hash_of("tabtool-about")
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use egui::{Context, ViewportId};

    use super::{AboutAction, wake_root};

    fn requested(action: Option<AboutAction>, close_requested: bool) -> bool {
        let ctx = Context::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&seen);
        ctx.set_repaint_observer(move |info| {
            record
                .lock()
                .expect("the observer lock is not poisoned")
                .push(info.viewport_id);
        });
        wake_root(&ctx, action.is_some(), close_requested);
        seen.lock()
            .expect("the observer lock is not poisoned")
            .contains(&ViewportId::ROOT)
    }

    #[test]
    fn a_link_and_a_close_wake_the_root() {
        assert!(requested(Some(AboutAction::Website), false));
        assert!(requested(None, true));
        assert!(!requested(None, false));
    }
}

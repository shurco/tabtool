//! The About content: identity, version, links and the bundled license.

use core::cell::RefCell;
use std::rc::Rc;

use tabtool_i18n::{Language, tr_in};
use tabtool_ui::about::{AboutAction, COPYRIGHT_KEY, DESCRIPTION_KEY, LICENSE_KEY, NAME_KEY, show};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::{Theme, UiHarness};

const LICENSE: &str = "The MIT License (MIT)\n\nCopyright (c) 2026 Dmitry Shurco";

fn harness(language: Language) -> (UiHarness<'static>, Rc<RefCell<Option<AboutAction>>>) {
    let action = Rc::new(RefCell::new(None));
    let recorded = Rc::clone(&action);
    let harness = UiHarness::wide(Theme::desktop_light(), move |ui| {
        if let Some(chosen) = show(ui, language, "1.2.3", LICENSE) {
            *recorded.borrow_mut() = Some(chosen);
        }
    });
    (harness, action)
}

#[test]
fn the_about_view_shows_identity_version_description_and_license() {
    let (mut harness, _action) = harness(Language::En);
    harness.run();

    assert!(harness.query_by_label("TabTool").is_some());
    assert!(harness.query_by_label("Version 1.2.3").is_some());
    assert!(harness.query_by_label(DESCRIPTION_KEY).is_some());
    assert!(harness.query_by_label("tabtool.shurco.app").is_some());
    assert!(harness.query_by_label("Rate TabTool").is_some());
    assert!(harness.query_by_label("MIT License").is_some());
    assert!(harness.query_by_label(COPYRIGHT_KEY).is_some());
    assert!(
        harness.query_by_label(LICENSE).is_some(),
        "the bundled license text is shown"
    );
    #[cfg(target_os = "macos")]
    assert!(
        harness
            .query_by_label(tabtool_ui::about::PLATFORM_DESCRIPTION_KEY)
            .is_some()
    );
}

#[test]
fn the_website_and_rate_links_report_their_actions() {
    let (mut harness, action) = harness(Language::En);
    harness.run();

    harness.get_by_label("tabtool.shurco.app").click();
    harness.run();
    assert_eq!(*action.borrow(), Some(AboutAction::Website));

    harness.get_by_label("Rate TabTool").click();
    harness.run();
    assert_eq!(*action.borrow(), Some(AboutAction::Rate));
}

#[test]
fn the_about_text_follows_the_language() {
    let (mut harness, _action) = harness(Language::De);
    harness.run();

    assert!(harness.query_by_label("Version 1.2.3").is_some());
    assert!(harness.query_by_label("TabTool bewerten").is_some());
    assert!(harness.query_by_label("MIT License").is_some());
}

#[test]
fn every_about_string_is_in_all_eleven_locales() {
    for language in Language::ALL.into_iter().skip(1) {
        for key in [
            NAME_KEY,
            DESCRIPTION_KEY,
            LICENSE_KEY,
            COPYRIGHT_KEY,
            "Version %@",
            "Rate TabTool",
        ] {
            assert!(
                tr_in(language, key).is_some(),
                "{key} is missing from {language:?}"
            );
        }
    }
}

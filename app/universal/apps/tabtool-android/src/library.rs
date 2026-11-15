//! The Android library view and its locale helpers.
//!
//! The library view holds the shared platform host as its document seam. The
//! app keeps the view across frames to preserve an open delete prompt or
//! rename field.

use std::rc::Rc;

use tabtool_core::Locale;
use tabtool_i18n::Language;
use tabtool_settings::Settings;
use tabtool_ui::mobile::{LibraryClock, LibraryView, MobileMetrics};

use crate::app::Host;

/// A library view over `host` with the current settings and layout.
pub fn new_view(
    host: Rc<dyn Host>,
    settings: &Settings,
    locale: Locale,
    clock: LibraryClock,
    metrics: MobileMetrics,
    language: Language,
) -> LibraryView {
    LibraryView::new(host, locale, clock, metrics, language)
        .new_document_configuration(settings.new_document_configuration())
}

/// The system locale, or English when the tag is not a valid ICU locale.
pub fn parse_locale(tag: &str) -> Locale {
    let tag = tag.replace('_', "-");
    Locale::try_from_str(&tag)
        .or_else(|_| Locale::try_from_str("en"))
        .expect("English is a valid ICU locale")
}

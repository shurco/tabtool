//! The Settings form: tabs, defaults, validation and immediate language switch.

use core::cell::RefCell;
use std::rc::Rc;

use tabtool_core::Encoding;
use tabtool_i18n::{Language, tr_in};
use tabtool_settings::{QuoteStyle, Settings};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::settings::{SettingsForm, SettingsTab, show};
use tabtool_ui::{Theme, UiHarness};

fn form(settings: Settings) -> Rc<RefCell<SettingsForm>> {
    Rc::new(RefCell::new(SettingsForm::new(settings)))
}

fn harness(form: Rc<RefCell<SettingsForm>>) -> UiHarness<'static> {
    UiHarness::wide(Theme::desktop_light(), move |ui| {
        let mut form = form.borrow_mut();
        let language = form.effective_language(Language::En);
        show(ui, &mut form, language);
    })
}

fn combo_value(harness: &UiHarness<'_>, name: &str) -> Option<String> {
    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, name)
        .value()
}

fn choose(harness: &mut UiHarness<'_>, picker: &str, option: &str) {
    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, picker)
        .click();
    harness.run();
    harness.get_by_label(option).click();
    harness.run();
}

fn tab(harness: &mut UiHarness<'_>, name: &str) {
    harness.get_by_label(name).click();
    harness.run();
}

#[test]
fn the_form_has_the_four_tabs_and_the_documented_defaults() {
    let state = form(Settings::default());
    let mut harness = harness(Rc::clone(&state));
    harness.run();

    for label in ["General", "New Document", "Grid", "Large Files"] {
        assert!(
            harness.query_by_label(label).is_some(),
            "missing tab {label}"
        );
    }
    assert_eq!(combo_value(&harness, "Language"), Some("System".into()));

    tab(&mut harness, "New Document");
    assert_eq!(
        combo_value(&harness, "Encoding"),
        Some("Unicode (UTF-8)".into())
    );
    assert_eq!(combo_value(&harness, "Separator"), Some("Comma (,)".into()));
    assert_eq!(combo_value(&harness, "Quote"), Some("Doubled".into()));
    assert_eq!(
        combo_value(&harness, "Decimal Mark"),
        Some("Dot (.)".into())
    );

    tab(&mut harness, "Grid");
    assert_eq!(combo_value(&harness, "Row Height"), Some("Default".into()));
    tab(&mut harness, "Large Files");
    assert_eq!(
        combo_value(&harness, "Large File Threshold"),
        Some("100 MiB".into())
    );
}

#[test]
fn choosing_a_language_re_renders_the_form_in_that_language() {
    let state = form(Settings::default());
    let mut harness = harness(Rc::clone(&state));
    harness.run();
    assert!(harness.query_by_label("General").is_some());

    choose(&mut harness, "Language", "Deutsch");

    assert_eq!(state.borrow().settings.app_language, Language::De);
    assert!(
        harness.query_by_label("Allgemein").is_some(),
        "the tab labels switch immediately"
    );
    assert!(harness.query_by_label("Neues Dokument").is_some());
    assert_eq!(combo_value(&harness, "Sprache"), Some("Deutsch".into()));
}

#[test]
fn the_general_tab_toggles_the_format_bar() {
    let state = form(Settings::default());
    let mut harness = harness(Rc::clone(&state));
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Show Format Bar")
        .click();
    harness.run();

    assert!(!state.borrow().settings.show_format_bar);
}

#[test]
fn the_grid_tab_changes_appearance_and_the_threshold_changes_large_files() {
    let state = form(Settings::default());
    let mut harness = harness(Rc::clone(&state));
    harness.run();
    tab(&mut harness, "Grid");

    choose(&mut harness, "Row Height", "Comfortable");
    assert_eq!(state.borrow().settings.grid_row_height, 32.0);
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Alternating Row Colors")
        .click();
    harness.run();
    assert!(state.borrow().settings.grid_alternating_rows);
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Show Row Numbers")
        .click();
    harness.run();
    assert!(!state.borrow().settings.grid_shows_row_numbers);

    tab(&mut harness, "Large Files");
    choose(&mut harness, "Large File Threshold", "1 GiB");
    assert_eq!(state.borrow().settings.large_file_threshold_mib, 1024);
}

#[test]
fn the_new_document_tab_edits_every_default() {
    let state = form(Settings::default());
    let mut harness = harness(Rc::clone(&state));
    harness.run();
    tab(&mut harness, "New Document");

    choose(&mut harness, "Encoding", "Western (Windows Latin 1)");
    choose(&mut harness, "Separator", "Tab (⇥)");
    choose(&mut harness, "Quote", "None");
    choose(&mut harness, "Decimal Mark", "Comma (,)");
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "First Row as Header")
        .click();
    harness.run();

    let settings = &state.borrow().settings;
    assert_eq!(settings.new_document_encoding, Encoding::Windows1252);
    assert_eq!(settings.new_document_separator, "\t");
    assert_eq!(settings.new_document_quote_style, QuoteStyle::None);
    assert_eq!(settings.new_document_decimal_mark, ",");
    assert!(settings.new_document_first_row_as_header);

    let configuration = settings.new_document_configuration();
    assert_eq!(configuration.encoding, Encoding::Windows1252);
    assert_eq!(configuration.column_separator, "\t");
    assert!(configuration.quote_character.is_empty());
    assert_eq!(configuration.decimal_mark, ",");
    assert!(configuration.first_row_as_header);
}

#[test]
fn invalid_stored_values_fall_back_to_the_offered_defaults() {
    let state = form(Settings {
        new_document_separator: "x".to_owned(),
        new_document_decimal_mark: "x".to_owned(),
        grid_row_height: 7.0,
        large_file_threshold_mib: 7,
        ..Settings::default()
    });
    let mut harness = harness(Rc::clone(&state));
    harness.run();

    tab(&mut harness, "New Document");
    assert_eq!(combo_value(&harness, "Separator"), Some("Comma (,)".into()));
    assert_eq!(
        combo_value(&harness, "Decimal Mark"),
        Some("Dot (.)".into())
    );

    tab(&mut harness, "Grid");
    assert_eq!(combo_value(&harness, "Row Height"), Some("Default".into()));

    tab(&mut harness, "Large Files");
    assert_eq!(
        combo_value(&harness, "Large File Threshold"),
        Some("100 MiB".into())
    );
}

#[test]
fn every_visible_settings_string_is_in_all_eleven_locales() {
    let keys = [
        "General",
        "New Document",
        "Grid",
        "Large Files",
        "Language",
        "System",
        "Show Format Bar",
        "Encoding",
        "Separator",
        "Quote",
        "Decimal Mark",
        "First Row as Header",
        "Doubled",
        "Backslash",
        "None",
        "These apply to new documents. Opened files keep the format detected from their content.",
        "Alternating Row Colors",
        "Row Height",
        "Compact",
        "Default",
        "Comfortable",
        "Show Row Numbers",
        "Large File Threshold",
        "Files at or above this size open in large file mode.",
        "Comma (,)",
        "Semicolon (;)",
        "Tab (⇥)",
        "Pipe (|)",
        "Dot (.)",
        "%lld MiB",
        "%lld GiB",
    ];
    for language in Language::ALL.into_iter().skip(1) {
        for key in keys {
            assert!(
                tr_in(language, key).is_some(),
                "{key} is missing from {language:?}"
            );
        }
    }
}

#[test]
fn the_tab_labels_come_from_the_catalog() {
    assert_eq!(SettingsTab::General.key(), "General");
    assert_eq!(SettingsTab::NewDocument.key(), "New Document");
    assert_eq!(SettingsTab::Grid.key(), "Grid");
    assert_eq!(SettingsTab::LargeFiles.key(), "Large Files");
    assert_eq!(SettingsTab::ALL.len(), 4);
}

//! The widget gallery, interaction and snapshot tests.

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::RichText;
use tabtool_i18n::{Language, tr};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::widgets::{
    AlertAction, BannerKind, ButtonKind, Menu, MenuItem, Picker, PushButton, banner, list_row,
    menu_bar, modal, search_field, segmented, sheet, status_line, tab_bar, text_field, toggle,
    toggle_enabled, toolbar, toolbar_separator,
};
use tabtool_ui::{Theme, UiHarness};

const EN: Language = Language::En;

fn en(key: &str) -> &str {
    tr(EN, key)
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).heading());
}

#[cfg(target_os = "macos")]
fn snapshots_enabled() -> bool {
    tabtool_ui::harness::render_snapshots_enabled()
}

/// Every widget in one deterministic screen, in the compact macOS density.
fn gallery(ui: &mut egui::Ui) {
    let theme = tabtool_ui::theme::current(ui.ctx());
    egui::Frame::new()
        .fill(theme.colors.window_background)
        .inner_margin(theme.metrics.window_padding)
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());

            menu_bar(
                ui,
                &[
                    Menu::new(
                        en("File"),
                        &[
                            MenuItem::new("close", en("Close")).shortcut("⌘W"),
                            MenuItem::new("save", en("Save As…")).shortcut("⇧⌘S"),
                        ],
                    ),
                    Menu::new(
                        en("Edit"),
                        &[MenuItem::new("copy", en("Copy")).shortcut("⌘C")],
                    ),
                ],
            );

            ui.add_space(10.0);
            toolbar(ui, |ui| {
                PushButton::new(en("Add Row"), ButtonKind::Plain).show(ui);
                PushButton::new(en("Add Column"), ButtonKind::Plain).show(ui);
                toolbar_separator(ui);
                PushButton::new(en("Delete"), ButtonKind::Plain).show(ui);
            });
            ui.add_space(10.0);
            ui.columns(2, |columns| {
                let (first, rest) = columns.split_at_mut(1);
                let left = &mut first[0];
                let right = &mut rest[0];

                heading(left, en("Add Row"));
                left.horizontal_wrapped(|ui| {
                    PushButton::new(en("Add Row"), ButtonKind::Bordered).show(ui);
                    PushButton::new(en("Add Column"), ButtonKind::Plain).show(ui);
                    PushButton::new(en("Save"), ButtonKind::Prominent).show(ui);
                    PushButton::new(en("Delete"), ButtonKind::Destructive).show(ui);
                });

                left.add_space(8.0);
                heading(left, en("Header"));
                let mut header = true;
                toggle(left, en("Header"), &mut header);

                left.add_space(8.0);
                heading(left, en("Row Height"));
                let mut density = 0;
                segmented(left, &[en("Compact"), en("Comfortable")], &mut density);

                left.add_space(8.0);
                heading(left, en("Settings"));
                let mut tab = 0;
                tab_bar(
                    left,
                    &[en("General"), en("Grid"), en("Large Files"), en("About")],
                    &mut tab,
                );

                left.add_space(8.0);
                heading(left, en("Encoding"));
                let encodings = [en("Unicode (UTF-8)"), en("Comma (,)"), en("Dot (.)")];
                Picker::new("encoding", en("Encoding"), encodings[0], &encodings)
                    .width(165.0)
                    .show(left);
                let mut search = String::new();
                search_field(left, en("Filters"), &mut search);
                let mut cell = String::from("value");
                text_field(left, en("Column"), &mut cell, en("Header"));

                right.add_space(8.0);
                heading(right, en("Documents"));
                list_row(right, en("Documents"), en("Settings"), true);
                list_row(right, en("Filters"), en("General"), false);

                right.add_space(8.0);
                heading(right, en("Format Not Applied"));
                banner(
                    right,
                    BannerKind::Info,
                    en("Loading…"),
                    en("The header is written only to the exported file."),
                    None,
                );
                banner(
                    right,
                    BannerKind::Warning,
                    en("Indexing is not finished. Save again when it completes."),
                    en("Files at or above this size open in large file mode."),
                    Some(en("Hide")),
                );
                banner(
                    right,
                    BannerKind::Error,
                    en("The file could not be opened"),
                    en("Check that the file still exists and that you have permission to open it."),
                    Some(en("Close")),
                );

                right.add_space(8.0);
                status_line(
                    right,
                    en("1,799 rows × 12 columns · Unicode (UTF-8) · Comma"),
                    en("Filters"),
                );
            });
        });
}

#[test]
fn gallery_renders_without_accessibility_errors() {
    let mut harness = UiHarness::wide(Theme::desktop_light(), gallery);

    harness.run();
}

#[cfg(target_os = "macos")]
#[test]
fn gallery_matches_the_light_snapshot() {
    if !snapshots_enabled() {
        return;
    }
    let mut harness = UiHarness::wide(Theme::desktop_light(), gallery);

    harness.run();
    harness.snapshot("gallery-light");
}

#[cfg(target_os = "macos")]
#[test]
fn gallery_matches_the_dark_snapshot() {
    if !snapshots_enabled() {
        return;
    }
    let mut harness = UiHarness::wide(Theme::desktop_dark(), gallery);

    harness.run();
    harness.snapshot("gallery-dark");
}

#[test]
fn toggle_click_changes_the_value() {
    let value = Rc::new(Cell::new(false));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let mut on = state.get();
        if toggle(ui, en("Header"), &mut on).changed() {
            state.set(on);
        }
    });

    harness.run();
    harness.get_by_label(en("Header")).click();
    harness.run();

    assert!(value.get());
}

#[test]
fn segmented_click_changes_the_selection() {
    let value = Rc::new(Cell::new(0));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let mut selected = state.get();
        segmented(ui, &[en("Compact"), en("Comfortable")], &mut selected);
        state.set(selected);
    });

    harness.run();
    harness.get_by_label(en("Comfortable")).click();
    harness.run();

    assert_eq!(value.get(), 1);
}

#[test]
fn picker_click_selects_an_option() {
    let options = [en("Unicode (UTF-8)"), en("Comma (,)"), en("Dot (.)")];
    let value = Rc::new(Cell::new(0));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let selected = options[state.get()];
        if let Some(index) = Picker::new("encoding", en("Encoding"), selected, &options).show(ui) {
            state.set(index);
        }
    });

    harness.run();
    harness.get_by_label(en("Encoding")).click();
    harness.run();
    harness.get_by_label(en("Comma (,)")).click();
    harness.run();

    assert_eq!(value.get(), 1);
}

#[test]
fn text_field_accepts_typed_text() {
    let value = Rc::new(RefCell::new(String::new()));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        text_field(ui, en("Column"), &mut state.borrow_mut(), en("Header"));
    });

    harness.run();
    harness.get_by_label(en("Column")).focus();
    harness.run();
    harness.get_by_label(en("Column")).type_text("42");
    harness.run();

    assert_eq!(value.borrow().as_str(), "42");
}

#[test]
fn menu_click_reports_the_item_id() {
    let chosen = Rc::new(RefCell::new(None::<String>));
    let state = Rc::clone(&chosen);
    let items = [MenuItem::new("close", en("Close")).shortcut("⌘W")];
    let menus = [Menu::new(en("File"), &items)];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(id) = menu_bar(ui, &menus) {
            *state.borrow_mut() = Some(id.to_owned());
        }
    });

    harness.run();
    harness.get_by_label(en("File")).click();
    harness.run();
    harness.get_by_label(en("Close")).click();
    harness.run();

    assert_eq!(*chosen.borrow(), Some("close".to_owned()));
}

#[test]
fn alert_click_reports_the_action_index() {
    let clicked = Rc::new(Cell::new(None));
    let state = Rc::clone(&clicked);
    let actions = [
        AlertAction::new(en("Delete"), ButtonKind::Destructive),
        AlertAction::new(en("Cancel"), ButtonKind::Bordered),
    ];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(index) = modal(
            ui,
            "delete-document",
            en("Delete Document?"),
            en("This cannot be undone."),
            &actions,
        ) {
            state.set(Some(index));
        }
    });

    harness.run();
    harness.get_by_label(en("Delete")).click();
    harness.run();

    assert_eq!(clicked.get(), Some(0));
}

#[test]
fn return_activates_the_default_action() {
    let clicked = Rc::new(Cell::new(None));
    let state = Rc::clone(&clicked);
    let actions = [
        AlertAction::new(en("Save"), ButtonKind::Prominent),
        AlertAction::new(en("Don’t Save"), ButtonKind::Destructive),
        AlertAction::new(en("Cancel"), ButtonKind::Bordered),
    ];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(index) = modal(
            ui,
            "quit-prompt",
            en("Quit"),
            en("Your changes will be lost if you don’t save them."),
            &actions,
        ) {
            state.set(Some(index));
        }
    });

    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();

    assert_eq!(clicked.get(), Some(0), "Return picks the first action");
}

#[test]
fn escape_reports_the_cancel_action() {
    let clicked = Rc::new(Cell::new(None));
    let state = Rc::clone(&clicked);
    let actions = [
        AlertAction::new(en("Save"), ButtonKind::Prominent),
        AlertAction::new(en("Don’t Save"), ButtonKind::Destructive),
        AlertAction::new(en("Cancel"), ButtonKind::Bordered),
    ];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(index) = modal(
            ui,
            "quit-prompt",
            en("Quit"),
            en("Your changes will be lost if you don’t save them."),
            &actions,
        ) {
            state.set(Some(index));
        }
    });

    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();

    assert_eq!(clicked.get(), Some(2), "Escape picks the last action");
}

#[test]
fn sheet_close_button_requests_a_close() {
    let closed = Rc::new(Cell::new(false));
    let state = Rc::clone(&closed);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let response = sheet(ui, "format", en("Format"), en("Close"), |ui| {
            ui.label(en("Encoding"));
        });
        if response.close_requested {
            state.set(true);
        }
    });

    harness.run();
    harness.get_by_label(en("Close")).click();
    harness.run();

    assert!(closed.get());
}

#[test]
fn banner_action_click_is_reported() {
    let clicked = Rc::new(Cell::new(false));
    let state = Rc::clone(&clicked);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let response = banner(
            ui,
            BannerKind::Warning,
            en("Indexing is not finished. Save again when it completes."),
            en("Files at or above this size open in large file mode."),
            Some(en("Hide")),
        );
        state.set(state.get() || response.action_clicked);
    });

    harness.run();
    harness.get_by_label(en("Hide")).click();
    harness.run();

    assert!(clicked.get());
}

#[test]
fn a_phone_banner_keeps_its_action_clear_of_the_wrapped_text() {
    let title = en("The file changed on disk. Reload to continue.");
    let message = en("Files at or above this size open in large file mode.");
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 240.0], move |ui| {
        banner(ui, BannerKind::Warning, title, message, Some(en("Reload")));
    });
    harness.run();

    let text = harness.get_by_label(title).rect();
    let action = harness.get_by_label(en("Reload")).rect();
    assert!(
        !text.intersects(action),
        "the wrapped text {text:?} must not run under the action {action:?}"
    );
    assert!(
        action.min.x >= 0.0 && action.max.x <= 360.0,
        "the action must stay on screen, got {action:?}"
    );
}

#[test]
fn list_row_click_is_reported() {
    let clicked = Rc::new(Cell::new(false));
    let state = Rc::clone(&clicked);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        state.set(state.get() || list_row(ui, en("Documents"), en("Settings"), false).clicked());
    });

    harness.run();
    harness.get_by_label(en("Documents")).click();
    harness.run();

    assert!(clicked.get());
}

#[test]
fn a_focused_list_row_activates_from_the_keyboard() {
    let clicked = Rc::new(Cell::new(false));
    let state = Rc::clone(&clicked);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let response = list_row(ui, en("Documents"), en("Settings"), false);
        state.set(state.get() || response.clicked());
    });
    harness.run();

    harness.get_by_label(en("Documents")).focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();

    assert!(clicked.get(), "Enter activates a focused list row");
}

#[test]
fn tab_bar_click_changes_the_selection() {
    let value = Rc::new(Cell::new(0));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let mut selected = state.get();
        tab_bar(ui, &[en("General"), en("Grid")], &mut selected);
        state.set(selected);
    });

    harness.run();
    harness.get_by_label(en("Grid")).click();
    harness.run();

    assert_eq!(value.get(), 1);
}

#[test]
fn a_disabled_button_does_not_fire() {
    let clicked = Rc::new(Cell::new(false));
    let state = Rc::clone(&clicked);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if PushButton::new(en("Close"), ButtonKind::Bordered)
            .enabled(false)
            .show(ui)
            .clicked()
        {
            state.set(true);
        }
    });

    harness.run();
    harness.get_by_label(en("Close")).click();
    harness.run();

    assert!(!clicked.get(), "a disabled button must not report a click");
}

#[cfg(target_os = "macos")]
#[test]
fn alert_matches_the_snapshot() {
    if !snapshots_enabled() {
        return;
    }
    let actions = [
        AlertAction::new(en("Delete"), ButtonKind::Destructive),
        AlertAction::new(en("Cancel"), ButtonKind::Bordered),
    ];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        ui.label(en("Documents"));
        modal(
            ui,
            "delete-document",
            en("Delete Document?"),
            en("This cannot be undone."),
            &actions,
        );
    });

    harness.run();
    harness.snapshot("alert-light");
}

#[cfg(target_os = "macos")]
#[test]
fn sheet_matches_the_snapshot() {
    if !snapshots_enabled() {
        return;
    }
    let mut harness = UiHarness::compact(Theme::desktop_dark(), |ui| {
        ui.label(en("Documents"));
        sheet(ui, "format", en("Format"), en("Close"), |ui| {
            let mut value = String::from("value");
            text_field(ui, en("Column"), &mut value, en("Header"));
        });
    });

    harness.run();
    harness.snapshot("sheet-dark");
}

#[test]
fn a_disabled_toggle_does_not_change() {
    let value = Rc::new(Cell::new(false));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let mut on = state.get();
        toggle_enabled(ui, en("Header"), &mut on, false);
        state.set(on);
    });

    harness.run();
    harness.get_by_label(en("Header")).click();
    harness.run();

    assert!(!value.get(), "a disabled toggle must not change");
}

#[test]
fn a_disabled_picker_does_not_open() {
    let options = [en("Unicode (UTF-8)"), en("Comma (,)"), en("Dot (.)")];
    let value = Rc::new(Cell::new(0));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        let selected = options[state.get()];
        if let Some(index) = Picker::new("encoding", en("Encoding"), selected, &options)
            .enabled(false)
            .show(ui)
        {
            state.set(index);
        }
    });

    harness.run();
    harness.get_by_label(en("Encoding")).click();
    harness.run();

    assert!(
        harness.query_by_label(en("Comma (,)")).is_none(),
        "a disabled picker must not open its popup"
    );
    assert_eq!(value.get(), 0);
}

#[test]
fn a_disabled_menu_item_does_not_fire() {
    let chosen = Rc::new(RefCell::new(None::<String>));
    let state = Rc::clone(&chosen);
    let items = [MenuItem::new("close", en("Close")).enabled(false)];
    let menus = [Menu::new(en("File"), &items)];
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        if let Some(id) = menu_bar(ui, &menus) {
            *state.borrow_mut() = Some(id.to_owned());
        }
    });

    harness.run();
    harness.get_by_label(en("File")).click();
    harness.run();
    harness.get_by_label(en("Close")).click();
    harness.run();

    assert_eq!(*chosen.borrow(), None);
}

#[test]
fn tab_focus_reaches_a_text_field() {
    let value = Rc::new(RefCell::new(String::new()));
    let state = Rc::clone(&value);
    let mut harness = UiHarness::compact(Theme::desktop_light(), move |ui| {
        text_field(ui, en("Column"), &mut state.borrow_mut(), en("Header"));
    });

    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();

    assert!(
        harness.get_by_label(en("Column")).is_focused(),
        "Tab must move keyboard focus to the text field"
    );
}

#[test]
fn a_focused_widget_draws_its_focus_ring() {
    let mut harness = UiHarness::compact(Theme::desktop_light(), |ui| {
        PushButton::new(en("Close"), ButtonKind::Bordered).show(ui);
    });

    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();

    assert!(
        harness.get_by_label(en("Close")).is_focused(),
        "Tab must move keyboard focus to the only button"
    );
    #[cfg(target_os = "macos")]
    if snapshots_enabled() {
        harness.snapshot("focus-ring-light");
    }
}

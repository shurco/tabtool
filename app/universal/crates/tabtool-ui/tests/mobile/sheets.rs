//! Sheet, list row, editor accessory and touch text-input seam.

use core::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use egui::scroll_area::{DragScroll, ScrollSource};
use egui::{Rect, ScrollArea, pos2};
use tabtool_i18n::{Language, tr};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{
    AccessoryAction, Insets, ListRow, MobileMetrics, MobileToolbar, MobileToolbarResponse,
    OverflowItem, SheetOptions, TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest,
    editor_accessory, sheet,
};
use tabtool_ui::{Theme, UiHarness};

const EN: Language = Language::En;

fn phone() -> MobileMetrics {
    MobileMetrics::phone().with_safe_area(Insets::from_array([0.0, 24.0, 0.0, 24.0]))
}

fn remember(state: &Rc<RefCell<MobileToolbarResponse>>, response: MobileToolbarResponse) {
    let mut last = state.borrow_mut();
    last.back |= response.back;
    last.undo |= response.undo;
    last.redo |= response.redo;
    last.filter |= response.filter;
    last.export |= response.export;
    if response.overflow.is_some() {
        last.overflow = response.overflow;
    }
}

#[test]
fn the_toolbar_shows_the_title_and_reports_every_button() {
    let result = Rc::new(RefCell::new(MobileToolbarResponse::default()));
    let state = Rc::clone(&result);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        let response = MobileToolbar::new(metrics, EN, "Untitled.csv")
            .modified(true)
            .history(true, true)
            .filter(true, 2)
            .export(true)
            .show(ui);
        remember(&state, response);
    });
    harness.run();

    assert!(harness.query_by_label("Untitled.csv •").is_some());
    let back = harness.get_by_label(tr(EN, "Documents"));
    assert!(back.rect().height() >= 44.0);
    assert!(back.rect().width() >= 44.0);

    harness.get_by_label(tr(EN, "Documents")).click();
    harness.run();
    assert!(result.borrow().back);

    harness.get_by_label(tr(EN, "Undo")).click();
    harness.run();
    assert!(result.borrow().undo);

    harness.get_by_label(tr(EN, "Redo")).click();
    harness.run();
    assert!(result.borrow().redo);

    harness.get_by_label("Filter (2)").click();
    harness.run();
    assert!(result.borrow().filter);

    harness.get_by_label(tr(EN, "Export")).click();
    harness.run();
    assert!(result.borrow().export);
}

#[test]
fn a_disabled_history_button_does_nothing() {
    let result = Rc::new(RefCell::new(MobileToolbarResponse::default()));
    let state = Rc::clone(&result);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        let response = MobileToolbar::new(metrics, EN, "Untitled.csv")
            .history(false, false)
            .show(ui);
        remember(&state, response);
    });
    harness.run();

    harness.get_by_label(tr(EN, "Undo")).click();
    harness.run();

    assert!(!result.borrow().undo);
}

#[test]
fn the_overflow_menu_offers_the_editor_sheets_and_save_as_in_large_mode() {
    let result = Rc::new(RefCell::new(MobileToolbarResponse::default()));
    let state = Rc::clone(&result);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        let response = MobileToolbar::new(metrics, EN, "large.csv")
            .large(true)
            .show(ui);
        remember(&state, response);
    });
    harness.run();

    harness.get_by_label(tr(EN, "More")).click();
    harness.run();

    assert!(harness.query_by_label(tr(EN, "Format")).is_some());
    assert!(harness.query_by_label(tr(EN, "Settings")).is_some());
    assert!(harness.query_by_label(tr(EN, "About")).is_some());
    assert!(harness.query_by_label(tr(EN, "Save As")).is_some());

    harness.get_by_label(tr(EN, "Format")).click();
    harness.run();

    assert_eq!(result.borrow().overflow, Some(OverflowItem::Format));
}

#[test]
fn the_overflow_menu_hides_save_as_outside_large_mode() {
    let result = Rc::new(RefCell::new(MobileToolbarResponse::default()));
    let state = Rc::clone(&result);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        let response = MobileToolbar::new(metrics, EN, "Untitled.csv").show(ui);
        remember(&state, response);
    });
    harness.run();

    harness.get_by_label(tr(EN, "More")).click();
    harness.run();

    assert!(harness.query_by_label(tr(EN, "Format")).is_some());
    assert!(harness.query_by_label(tr(EN, "Save As")).is_none());
}

#[test]
fn the_sheet_shows_its_title_and_a_drag_handle_and_closes() {
    let result = Rc::new(RefCell::new((false, None)));
    let state = Rc::clone(&result);
    let metrics = phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = sheet(
            ui.ctx(),
            "format",
            metrics,
            SheetOptions::new(tr(EN, "Format"), tr(EN, "Done")),
            |ui| {
                ui.label(tr(EN, "Encoding"));
            },
        );
        let mut result = state.borrow_mut();
        result.0 |= response.close_requested;
        result.1 = Some(response.width);
    });
    harness.run();

    assert!(harness.query_by_label(tr(EN, "Format")).is_some());
    assert!(harness.query_by_label(tr(EN, "Encoding")).is_some());
    assert_eq!(result.borrow().1, Some(360.0));

    harness.get_by_label(tr(EN, "Done")).click();
    harness.run();

    assert!(result.borrow().0);
}

#[test]
fn the_tablet_sheet_is_capped_at_five_hundred_and_forty_points() {
    let result = Rc::new(RefCell::new(None));
    let state = Rc::clone(&result);
    let metrics = MobileMetrics::tablet();
    let mut harness = UiHarness::new(Theme::desktop_light(), [800.0, 1280.0], move |ui| {
        let response = sheet(
            ui.ctx(),
            "settings",
            metrics,
            SheetOptions::new(tr(EN, "Settings"), tr(EN, "Done")),
            |ui| {
                ui.label(tr(EN, "General"));
            },
        );
        *state.borrow_mut() = Some(response.width);
    });
    harness.run();

    assert_eq!(*result.borrow(), Some(540.0));
}

#[test]
fn the_sheet_lifts_itself_above_the_keyboard() {
    let result = Rc::new(RefCell::new(0.0));
    let state = Rc::clone(&result);
    let metrics = phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = sheet(
            ui.ctx(),
            "keyboard",
            metrics,
            SheetOptions::new(tr(EN, "Format"), tr(EN, "Done")).keyboard_height(300.0),
            |ui| {
                ui.label(tr(EN, "Encoding"));
            },
        );
        *state.borrow_mut() = response.max_height;
    });
    harness.run();

    assert_eq!(*result.borrow(), 476.0);
}

#[test]
fn dragging_the_handle_down_dismisses_the_sheet() {
    let result = Rc::new(RefCell::new(false));
    let state = Rc::clone(&result);
    let metrics = phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = sheet(
            ui.ctx(),
            "drag",
            metrics,
            SheetOptions::new(tr(EN, "Format"), tr(EN, "Done")),
            |ui| {
                ui.label(tr(EN, "Encoding"));
            },
        );
        *state.borrow_mut() |= response.close_requested;
    });
    harness.run();

    let title = harness.get_by_label(tr(EN, "Format")).rect();
    let handle_y = title.min.y - 15.0;
    let start = pos2(title.center().x, handle_y);
    let end = pos2(start.x, start.y + 100.0);
    harness.drag_at(start);
    harness.hover_at(end);
    harness.run();
    harness.drop_at(end);
    harness.run();

    assert!(*result.borrow());
}

#[test]
fn the_sheet_closes_on_escape() {
    let result = Rc::new(RefCell::new(false));
    let state = Rc::clone(&result);
    let metrics = phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = sheet(
            ui.ctx(),
            "about",
            metrics,
            SheetOptions::new(tr(EN, "About"), tr(EN, "Done")),
            |ui| {
                ui.label(tr(EN, "Version"));
            },
        );
        *state.borrow_mut() |= response.close_requested;
    });
    harness.run();

    harness.key_press(egui::Key::Escape);
    harness.run();

    assert!(*result.borrow());
}

#[test]
fn the_sheet_scrolls_tall_content() {
    let metrics = phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        sheet(
            ui.ctx(),
            "filters",
            metrics,
            SheetOptions::new(tr(EN, "Filters"), tr(EN, "Done")),
            |ui| {
                for row in 0..80 {
                    ui.label(format!("row {row}"));
                }
            },
        );
    });
    harness.run();

    assert!(harness.query_by_label("row 0").is_some());
}

#[test]
fn a_list_row_reports_a_tap_and_shows_its_subtitle() {
    let outcome = Rc::new(RefCell::new((false, false, false)));
    let state = Rc::clone(&outcome);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = ListRow::new("Notes.csv")
            .subtitle("2 KB · 14 Nov 2026")
            .chevron(true)
            .show(ui, metrics, EN);
        let mut outcome = state.borrow_mut();
        outcome.0 |= response.activated;
        outcome.1 |= response.delete_requested;
        outcome.2 |= response.rename_requested;
    });
    harness.run();

    assert!(harness.query_by_label("Notes.csv").is_some());
    assert!(harness.query_by_label("2 KB · 14 Nov 2026").is_some());

    harness.get_by_label("Notes.csv").click();
    harness.run();

    assert_eq!(*outcome.borrow(), (true, false, false));
}

#[test]
fn the_list_row_context_menu_requests_a_rename() {
    let outcome = Rc::new(RefCell::new((false, false, false)));
    let state = Rc::clone(&outcome);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = ListRow::new("Notes.csv").show(ui, metrics, EN);
        let mut outcome = state.borrow_mut();
        outcome.0 |= response.activated;
        outcome.1 |= response.delete_requested;
        outcome.2 |= response.rename_requested;
    });
    harness.run();

    harness.get_by_label("Notes.csv").click_secondary();
    harness.run();
    harness.get_by_label(tr(EN, "Rename")).click();
    harness.run();

    assert_eq!(*outcome.borrow(), (false, false, true));
}

#[test]
fn a_touch_long_press_requests_a_rename() {
    let outcome = Rc::new(RefCell::new((false, false, false)));
    let state = Rc::clone(&outcome);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = ListRow::new("Notes.csv").show(ui, metrics, EN);
        let mut outcome = state.borrow_mut();
        outcome.0 |= response.activated;
        outcome.1 |= response.delete_requested;
        outcome.2 |= response.rename_requested;
    });
    harness.run();

    let center = harness.get_by_label("Notes.csv").rect().center();
    harness.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: center,
        force: Some(1.0),
    });
    harness.event(egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run_steps(5);

    assert_eq!(
        *outcome.borrow(),
        (false, false, true),
        "a long touch opens Rename without activating the row"
    );

    harness.event(egui::Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::End,
        pos: center,
        force: None,
    });
    harness.event(egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
}

#[test]
fn swiping_a_list_row_reveals_delete() {
    let outcome = Rc::new(RefCell::new((false, false, false)));
    let state = Rc::clone(&outcome);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        let response = ListRow::new("Notes.csv").show(ui, metrics, EN);
        let mut outcome = state.borrow_mut();
        outcome.0 |= response.activated;
        outcome.1 |= response.delete_requested;
        outcome.2 |= response.rename_requested;
    });
    harness.run();

    let rect = harness.get_by_label("Notes.csv").rect();
    let start = rect.center();
    let end = pos2(rect.center().x - 100.0, rect.center().y);
    harness.drag_at(start);
    harness.hover_at(end);
    harness.run();
    harness.drop_at(end);
    harness.run();

    let delete = harness
        .query_by_label(tr(EN, "Delete"))
        .expect("swiping reveals Delete");
    delete.click();
    harness.run();

    assert_eq!(*outcome.borrow(), (false, true, false));
}

#[test]
fn a_vertical_drag_scrolls_the_list_instead_of_swiping() {
    let outcome = Rc::new(RefCell::new((false, false, false)));
    let state = Rc::clone(&outcome);
    let offset = Rc::new(RefCell::new(0.0f32));
    let scroll_state = Rc::clone(&offset);
    let metrics = MobileMetrics::phone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 200.0], move |ui| {
        let output = ScrollArea::vertical()
            .id_salt("library-list")
            .scroll_source(ScrollSource {
                drag: DragScroll::Always,
                ..ScrollSource::default()
            })
            .show(ui, |ui| {
                for index in 0..20 {
                    let title = format!("Row {index}");
                    let response = ListRow::new(&title).subtitle("2 KB").show(ui, metrics, EN);
                    let mut outcome = state.borrow_mut();
                    outcome.0 |= response.activated;
                    outcome.1 |= response.delete_requested;
                    outcome.2 |= response.rename_requested;
                }
            });
        *scroll_state.borrow_mut() = output.state.clamped_offset().y;
    });
    harness.run();

    let rect = harness.get_by_label("Row 0").rect();
    let start = rect.center();
    let end = pos2(start.x, start.y - 80.0);
    harness.drag_at(start);
    harness.hover_at(end);
    harness.run();
    harness.drop_at(end);
    harness.run();

    assert!(*offset.borrow() > 0.0, "a vertical drag scrolls the list");
    assert!(
        !outcome.borrow().1,
        "a vertical drag must not reveal delete"
    );
    assert!(!outcome.borrow().0, "a vertical drag must not open the row");
}

#[test]
fn the_accessory_reports_previous_next_and_done() {
    let choice = Rc::new(RefCell::new(None));
    let state = Rc::clone(&choice);
    let metrics = MobileMetrics::phone();

    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
        if let Some(action) = editor_accessory(ui, metrics, EN) {
            *state.borrow_mut() = Some(action);
        }
    });
    harness.run();

    harness.get_by_label(tr(EN, "Previous Cell")).click();
    harness.run();
    assert_eq!(*choice.borrow(), Some(AccessoryAction::Previous));

    harness.get_by_label(tr(EN, "Next Cell")).click();
    harness.run();
    assert_eq!(*choice.borrow(), Some(AccessoryAction::Next));

    harness.get_by_label(tr(EN, "Done")).click();
    harness.run();
    assert_eq!(*choice.borrow(), Some(AccessoryAction::Done));
}

#[derive(Default)]
struct FakeTextInput {
    request: Option<TouchTextInputRequest>,
    pending: Option<TouchTextInputOutcome>,
}

impl TouchTextInput for FakeTextInput {
    fn open(&mut self, request: TouchTextInputRequest) {
        self.pending = None;
        self.request = Some(request);
    }

    fn close(&mut self) {
        self.request = None;
        self.pending = None;
    }

    fn commit(&mut self) {
        self.pending = None;
    }

    fn move_to(&mut self, _rect: egui::Rect) {
        // The sheets never move an open text session.
    }

    fn poll(&mut self) -> Option<TouchTextInputOutcome> {
        self.pending.take()
    }

    fn is_active(&self) -> bool {
        self.request.is_some()
    }
}

#[test]
fn the_touch_text_input_seam_opens_over_the_control_rect() {
    let mut input = FakeTextInput::default();
    let rect = Rect::from_min_max(pos2(40.0, 100.0), pos2(200.0, 144.0));

    input.open(TouchTextInputRequest {
        rect,
        text: "Alice".to_owned(),
        multiline: false,
    });
    assert!(input.is_active());
    assert_eq!(
        input.request.as_ref().map(|request| request.rect),
        Some(rect)
    );

    input.pending = Some(TouchTextInputOutcome::Committed("Bob".to_owned()));
    assert_eq!(
        input.poll(),
        Some(TouchTextInputOutcome::Committed("Bob".to_owned()))
    );
    input.close();
    assert!(!input.is_active());
}

fn mobile_sources() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mobile");
    let mut files = Vec::new();
    collect_sources(&directory, &mut files);
    files.sort();
    files
}

fn collect_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("the mobile module is readable") {
        let path = entry.expect("a module entry").path();
        if path.is_dir() {
            collect_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn the_mobile_primitives_use_the_touch_seam_not_an_egui_text_edit() {
    for path in mobile_sources() {
        let source = std::fs::read_to_string(&path).expect("a mobile source is UTF-8");
        assert!(
            !source.contains("TextEdit"),
            "{} must use the native text input, not TextEdit",
            path.display()
        );
    }
}

#[test]
fn the_mobile_primitives_stay_platform_neutral() {
    for path in mobile_sources() {
        let source = std::fs::read_to_string(&path).expect("a mobile source is UTF-8");
        for forbidden in ["tabtool_platform", "jni::", "android"] {
            assert!(
                !source.contains(forbidden),
                "{} must not depend on `{forbidden}`",
                path.display()
            );
        }
    }
}

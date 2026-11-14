//! Sheet, list row, editor accessory, touch text-input seam and the five
//! touch sheets.

use core::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use egui::accesskit::Role;
use egui::scroll_area::{DragScroll, ScrollSource};
use egui::{Rect, ScrollArea, pos2};
use tabtool_core::{CsvConfiguration, CsvError, Encoding, FilterOperator};
use tabtool_document::{Document, DocumentError, LargeStatus, StdStorage};
use tabtool_i18n::{Language, tr, tr_in};
use tabtool_settings::Settings;
use tabtool_ui::about::AboutAction;
use tabtool_ui::document::alert_message;
use tabtool_ui::harness::kittest::{NodeT, Queryable};
use tabtool_ui::mobile::sheets::{
    CopyResult, ExportSheet, FiltersSheet, FormatSheet, SAVED_COPY_KEY, SaveCopy, SheetStyle,
    about_sheet, saved_copy_message, settings_sheet,
};
use tabtool_ui::mobile::{
    AccessoryAction, BackOutcome, DocumentFiles, DocumentId, Insets, ListRow, MobileMetrics,
    MobileToolbar, MobileToolbarResponse, NavStack, OverflowItem, Screen, SheetKind, SheetOptions,
    TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest, editor_accessory, sheet,
};
use tabtool_ui::{Theme, UiHarness};
use tempfile::TempDir;

use crate::support::{FakeTouchInput, multiline_document, small_document};

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
        if let Some(action) = editor_accessory(ui, metrics, EN, true) {
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

// -- Touch sheets ---------------------------------------------------------
//
// Format, Filters, Settings, About and Export share the mobile `sheet`
// container. These tests drive them through the accessibility tree, the same
// way the touch editor is tested.

fn sheet_style() -> SheetStyle {
    SheetStyle::new(phone(), EN)
}

fn header_configuration() -> CsvConfiguration {
    CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    }
}

fn file_document(
    directory: &TempDir,
    name: &str,
    bytes: &[u8],
    configuration: CsvConfiguration,
) -> Document {
    let path = directory.path().join(name);
    std::fs::write(&path, bytes).expect("the fixture file is written");
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(bytes, Some(&path), Some(configuration), false)
        .expect("the fixture parses");
    document
}

struct FormatFixture {
    ui: UiHarness<'static>,
    document: Rc<RefCell<Document>>,
    closed: Rc<Cell<bool>>,
}

impl FormatFixture {
    fn new(document: Document, prefers_chinese: bool) -> Self {
        let document = Rc::new(RefCell::new(document));
        let sheet = Rc::new(RefCell::new(FormatSheet::new()));
        let closed = Rc::new(Cell::new(false));
        let ui_document = Rc::clone(&document);
        let ui_sheet = Rc::clone(&sheet);
        let ui_closed = Rc::clone(&closed);
        let ui = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
            let mut document = ui_document.borrow_mut();
            let response = ui_sheet.borrow_mut().show(
                ui,
                sheet_style(),
                &mut document,
                &StdStorage,
                prefers_chinese,
            );
            ui_closed.set(ui_closed.get() || response.close_requested);
        });
        Self {
            ui,
            document,
            closed,
        }
    }

    fn run(&mut self) {
        self.ui.run();
    }

    fn click(&mut self, label: &str) {
        self.ui.get_by_label(label).click();
        self.run();
    }

    fn click_button(&mut self, label: &str) {
        self.ui.get_by_role_and_label(Role::Button, label).click();
        self.run();
    }

    fn choose(&mut self, picker: &str, option: &str) {
        self.ui
            .get_by_role_and_label(Role::ComboBox, picker)
            .click();
        self.run();
        self.ui.get_by_label(option).click();
        self.run();
    }
}

impl core::ops::Deref for FormatFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for FormatFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

struct FiltersFixture {
    ui: UiHarness<'static>,
    document: Rc<RefCell<Document>>,
    input: FakeTouchInput,
    closed: Rc<Cell<bool>>,
}

impl FiltersFixture {
    fn new(document: Document) -> Self {
        let document = Rc::new(RefCell::new(document));
        let sheet = Rc::new(RefCell::new(FiltersSheet::new()));
        let input = FakeTouchInput::default();
        let closed = Rc::new(Cell::new(false));
        let ui_document = Rc::clone(&document);
        let ui_sheet = Rc::clone(&sheet);
        let ui_input = input.clone();
        let ui_closed = Rc::clone(&closed);
        let ui = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
            let mut document = ui_document.borrow_mut();
            let mut input = ui_input.clone();
            let response = ui_sheet
                .borrow_mut()
                .show(ui, sheet_style(), &mut document, &mut input);
            ui_closed.set(ui_closed.get() || response.close_requested);
        });
        Self {
            ui,
            document,
            input,
            closed,
        }
    }

    fn run(&mut self) {
        self.ui.run();
    }

    fn click(&mut self, label: &str) {
        self.ui.get_by_label(label).click();
        self.run();
    }

    fn click_button(&mut self, label: &str) {
        self.ui.get_by_role_and_label(Role::Button, label).click();
        self.run();
    }

    fn type_value(&mut self, text: &str) {
        self.ui
            .get_by_role_and_label(Role::TextInput, tr(EN, "Value"))
            .click();
        self.run();
        self.input.set_draft(text);
        self.input
            .finish(TouchTextInputOutcome::Committed(text.to_owned()));
        self.run();
    }
}

impl core::ops::Deref for FiltersFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for FiltersFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

#[derive(Clone, Debug)]
struct ExportCall {
    source: PathBuf,
    name: String,
    bytes: Vec<u8>,
}

struct ExportFiles {
    calls: RefCell<Vec<ExportCall>>,
    success: Cell<bool>,
    defer_once: Cell<bool>,
    in_progress: Cell<bool>,
}

impl ExportFiles {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            success: Cell::new(true),
            defer_once: Cell::new(false),
            in_progress: Cell::new(false),
        }
    }

    fn calls(&self) -> Vec<ExportCall> {
        self.calls.borrow().clone()
    }

    fn defer_once(&self) {
        self.defer_once.set(true);
    }

    fn finish_pending(&self) {
        self.in_progress.set(false);
    }
}

impl DocumentFiles for ExportFiles {
    fn documents_dir(&self) -> PathBuf {
        std::env::temp_dir()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn existing_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn create_document(&self, _name: &str, _data: &[u8]) -> std::io::Result<PathBuf> {
        Err(std::io::Error::other("the export fake does not create"))
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        None
    }

    fn delete(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> bool {
        let bytes = std::fs::read(source).unwrap_or_default();
        self.calls.borrow_mut().push(ExportCall {
            source: source.to_path_buf(),
            name: suggested_name.to_owned(),
            bytes,
        });
        if self.defer_once.replace(false) {
            self.in_progress.set(true);
            return false;
        }
        self.success.get()
    }

    fn export_in_progress(&self) -> bool {
        self.in_progress.get()
    }
}

struct ExportFixture {
    ui: UiHarness<'static>,
    document: Rc<RefCell<Document>>,
    files: Rc<ExportFiles>,
    sheet: Rc<RefCell<ExportSheet>>,
    closed: Rc<Cell<bool>>,
}

impl ExportFixture {
    fn new(document: Document) -> Self {
        let sheet = Rc::new(RefCell::new(ExportSheet::new(&document)));
        let document = Rc::new(RefCell::new(document));
        let files = Rc::new(ExportFiles::new());
        let closed = Rc::new(Cell::new(false));
        let ui_sheet = Rc::clone(&sheet);
        let ui_document = Rc::clone(&document);
        let ui_files = Rc::clone(&files);
        let ui_closed = Rc::clone(&closed);
        let ui = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
            let document = ui_document.borrow();
            let response =
                ui_sheet
                    .borrow_mut()
                    .show(ui, sheet_style(), &document, ui_files.as_ref());
            ui_closed.set(ui_closed.get() || response.close_requested);
        });
        Self {
            ui,
            document,
            files,
            sheet,
            closed,
        }
    }

    fn run(&mut self) {
        self.ui.run();
    }

    fn click_button(&mut self, label: &str) {
        self.ui.get_by_role_and_label(Role::Button, label).click();
        self.run();
    }

    fn choose(&mut self, picker: &str, option: &str) {
        self.ui
            .get_by_role_and_label(Role::ComboBox, picker)
            .click();
        self.run();
        self.ui.get_by_label(option).click();
        self.run();
    }
}

impl core::ops::Deref for ExportFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for ExportFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

struct SettingsFixture {
    ui: UiHarness<'static>,
    settings: Rc<RefCell<Settings>>,
    about: Rc<Cell<bool>>,
    closed: Rc<Cell<bool>>,
}

impl SettingsFixture {
    fn new(settings: Settings) -> Self {
        let settings = Rc::new(RefCell::new(settings));
        let about = Rc::new(Cell::new(false));
        let closed = Rc::new(Cell::new(false));
        let ui_settings = Rc::clone(&settings);
        let ui_about = Rc::clone(&about);
        let ui_closed = Rc::clone(&closed);
        let ui = UiHarness::new(Theme::desktop_light(), [360.0, 1200.0], move |ui| {
            let mut settings = ui_settings.borrow_mut();
            let language = match settings.app_language {
                Language::System => EN,
                language => language,
            };
            let response = settings_sheet(ui, SheetStyle::new(phone(), language), &mut settings);
            ui_about.set(ui_about.get() || response.about);
            ui_closed.set(ui_closed.get() || response.close_requested);
        });
        Self {
            ui,
            settings,
            about,
            closed,
        }
    }

    fn run(&mut self) {
        self.ui.run();
    }

    fn click(&mut self, label: &str) {
        self.ui.get_by_label(label).click();
        self.run();
    }

    fn click_button(&mut self, label: &str) {
        self.ui.get_by_role_and_label(Role::Button, label).click();
        self.run();
    }
}

impl core::ops::Deref for SettingsFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for SettingsFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

struct AboutFixture {
    ui: UiHarness<'static>,
    action: Rc<Cell<Option<AboutAction>>>,
    closed: Rc<Cell<bool>>,
}

impl AboutFixture {
    fn new() -> Self {
        let action = Rc::new(Cell::new(None));
        let closed = Rc::new(Cell::new(false));
        let ui_action = Rc::clone(&action);
        let ui_closed = Rc::clone(&closed);
        let ui = UiHarness::new(Theme::desktop_light(), [360.0, 800.0], move |ui| {
            let response = about_sheet(
                ui,
                sheet_style(),
                "0.1.0",
                "MIT License\n\nPermission is hereby granted.",
            );
            if let Some(chosen) = response.action {
                ui_action.set(Some(chosen));
            }
            ui_closed.set(ui_closed.get() || response.close_requested);
        });
        Self { ui, action, closed }
    }

    fn run(&mut self) {
        self.ui.run();
    }

    fn click(&mut self, label: &str) {
        self.ui.get_by_label(label).click();
        self.run();
    }

    fn click_button(&mut self, label: &str) {
        self.ui.get_by_role_and_label(Role::Button, label).click();
        self.run();
    }
}

impl core::ops::Deref for AboutFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for AboutFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

// -- Format sheet ---------------------------------------------------------

#[test]
fn the_format_sheet_shows_every_control_and_closes() {
    let mut fixture = FormatFixture::new(small_document(), false);
    fixture.run();

    for label in ["Encoding", "Separator", "Quote", "Decimal"] {
        assert!(
            fixture
                .query_by_role_and_label(Role::ComboBox, label)
                .is_some(),
            "missing {label} picker"
        );
    }
    assert!(
        fixture
            .query_by_role_and_label(Role::CheckBox, tr(EN, "First Row as Header"))
            .is_some()
    );
    assert!(
        fixture
            .query_by_role_and_label(Role::Button, tr(EN, "Detect Automatically"))
            .is_some()
    );

    fixture.click(tr(EN, "Done"));
    assert!(fixture.closed.get());
}

#[test]
fn the_format_sheet_disables_the_decimal_mark_in_large_mode() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = crate::support::large_document(&directory);
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    assert!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Decimal")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        !fixture
            .get_by_role_and_label(Role::ComboBox, "Encoding")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        !fixture
            .get_by_role_and_label(Role::Button, tr(EN, "Detect Automatically"))
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn detect_is_disabled_without_a_file_path() {
    let mut fixture = FormatFixture::new(small_document(), false);
    fixture.run();

    assert!(
        fixture
            .get_by_role_and_label(Role::Button, tr(EN, "Detect Automatically"))
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn a_dirty_parsing_change_asks_before_discarding_edits() {
    let directory = TempDir::new().expect("a temporary directory");
    let mut document = file_document(
        &directory,
        "notes.csv",
        b"Name,Amount\nAlice,1.5\nBob,2\n",
        header_configuration(),
    );
    document.set_decimal_mark(",");
    let before = document.configuration().column_separator.clone();
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    fixture.choose("Separator", "Semicolon (;)");
    assert!(
        fixture
            .query_by_label(tr(EN, "Re-read the file and discard unsaved edits?"))
            .is_some()
    );
    fixture.click_button(tr(EN, "Cancel"));
    assert_eq!(
        fixture.document.borrow().configuration().column_separator,
        before
    );

    fixture.choose("Separator", "Semicolon (;)");
    fixture.click_button(tr(EN, "Re-read"));
    assert_eq!(
        fixture.document.borrow().configuration().column_separator,
        ";"
    );
    assert!(fixture.document.borrow().table().is_some());
}

#[test]
fn a_failed_reparse_keeps_the_table_and_reports_format_not_applied() {
    let directory = TempDir::new().expect("a temporary directory");
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let document = file_document(
        &directory,
        "legacy.csv",
        b"Name,Amount\nAlice,\xFF\n",
        configuration,
    );
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    fixture.choose("Encoding", "Unicode (UTF-8)");

    assert!(
        fixture
            .query_by_label(tr(EN, "Format Not Applied"))
            .is_some()
    );
    let failed = fixture.document.borrow();
    assert_eq!(failed.configuration().encoding, Encoding::Windows1252);
    let table = failed.table().expect("the table stays");
    assert_eq!(table.cell(0, 0), "Alice");
}

#[test]
fn detect_reads_the_file_and_applies_the_detected_format() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = file_document(
        &directory,
        "detected.csv",
        b"Name;Amount\nAlice;1\n",
        header_configuration(),
    );
    assert_eq!(document.configuration().column_separator, ",");
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    fixture.click_button(tr(EN, "Detect Automatically"));

    assert_eq!(
        fixture.document.borrow().configuration().column_separator,
        ";"
    );
}

#[test]
fn a_sheet_does_not_stop_a_large_save() {
    let directory = TempDir::new().expect("a temporary directory");
    let mut document = crate::support::large_document(&directory);
    document.force_saving_state_for_test(0.5);
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    assert_eq!(
        fixture.document.borrow().large_status(),
        LargeStatus::Saving(0.5)
    );

    fixture.click(tr(EN, "Done"));
    assert!(fixture.closed.get());
    assert_eq!(
        fixture.document.borrow().large_status(),
        LargeStatus::Saving(0.5)
    );
}

#[test]
fn a_sheet_does_not_hide_indexing_progress() {
    let directory = TempDir::new().expect("a temporary directory");
    let mut document = crate::support::large_document(&directory);
    document.force_indexing_state_for_test(0.4);
    document.poll_large_indexing();
    let mut fixture = FormatFixture::new(document, false);
    fixture.run();

    assert_eq!(
        fixture.document.borrow().large_status(),
        LargeStatus::Indexing(0.4)
    );

    fixture.click(tr(EN, "Done"));
    assert_eq!(
        fixture.document.borrow().large_status(),
        LargeStatus::Indexing(0.4)
    );
}

// -- Filters sheet --------------------------------------------------------

#[test]
fn the_filters_sheet_adds_and_removes_criterion_cards() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    assert_eq!(
        fixture.document.borrow().row_filter().criteria.len(),
        1,
        "the sheet starts with one card"
    );
    fixture.click_button(tr(EN, "Add Criterion"));
    assert_eq!(fixture.document.borrow().row_filter().criteria.len(), 2);

    fixture
        .query_all_by_label(tr(EN, "Remove Criterion"))
        .next()
        .expect("the first card has a remove button")
        .click();
    fixture.run();
    assert_eq!(fixture.document.borrow().row_filter().criteria.len(), 1);
}

#[test]
fn the_value_field_uses_the_native_text_input() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::TextInput, tr(EN, "Value"))
        .click();
    fixture.run();
    let request = fixture.input.request().expect("the native editor opens");
    assert!(!request.multiline);
    assert!(request.rect.height() >= 44.0);

    fixture.input.set_draft("Alice");
    fixture
        .input
        .finish(TouchTextInputOutcome::Committed("Alice".to_owned()));
    fixture.run();

    assert_eq!(
        fixture.document.borrow().row_filter().criteria[0].value,
        "Alice"
    );
}

#[test]
fn applying_filters_uses_the_draft_and_clear_resets_it() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture.type_value("Ali");
    fixture.click_button(tr(EN, "Apply"));

    assert_eq!(
        fixture.document.borrow().row_filter().visible_rows,
        Some(vec![0])
    );
    assert_eq!(fixture.document.borrow().row_filter().criteria.len(), 1);

    fixture.click_button(tr(EN, "Clear"));
    assert_eq!(fixture.document.borrow().row_filter().visible_rows, None);
    assert!(!fixture.document.borrow().row_filter().is_filtering());
    assert_eq!(fixture.document.borrow().row_filter().criteria.len(), 1);
    assert!(
        fixture.document.borrow().row_filter().criteria[0]
            .value
            .is_empty()
    );
}

#[test]
fn applying_filters_commits_an_open_value_first() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::TextInput, tr(EN, "Value"))
        .click();
    fixture.run();
    fixture.input.set_draft("Ali");
    fixture.click_button(tr(EN, "Apply"));

    assert!(fixture.input.commits() > 0, "Apply commits the open field");
    assert_eq!(
        fixture.document.borrow().row_filter().criteria[0].value,
        "Ali"
    );
    assert_eq!(
        fixture.document.borrow().row_filter().visible_rows,
        Some(vec![0])
    );
}

#[test]
fn an_invalid_regex_shows_an_error_and_does_not_filter() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture.document.borrow_mut().row_filter_mut().criteria[0].op = FilterOperator::MatchesRegex;
    fixture.run();
    fixture.type_value("(");

    assert_eq!(
        fixture.document.borrow().row_filter().criteria[0].op,
        FilterOperator::MatchesRegex
    );
    assert_eq!(
        fixture.document.borrow().row_filter().criteria[0].value,
        "("
    );
    assert!(
        fixture
            .query_by_label(tr(EN, "Invalid regular expression"))
            .is_some()
    );
    assert!(
        fixture
            .get_by_role_and_label(Role::Button, tr(EN, "Apply"))
            .accesskit_node()
            .is_disabled()
    );
    assert_eq!(fixture.document.borrow().row_filter().visible_rows, None);
}

#[test]
fn closing_the_filters_sheet_keeps_the_draft() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture.type_value("Bob");
    fixture.click(tr(EN, "Done"));

    assert!(fixture.closed.get());
    assert_eq!(
        fixture.document.borrow().row_filter().criteria[0].value,
        "Bob"
    );
    assert_eq!(fixture.document.borrow().row_filter().visible_rows, None);
}

#[test]
fn closing_the_filters_sheet_discards_an_open_native_edit() {
    let mut fixture = FiltersFixture::new(small_document());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::TextInput, tr(EN, "Value"))
        .click();
    fixture.run();
    assert!(fixture.input.is_active());

    fixture.click(tr(EN, "Done"));
    assert!(!fixture.input.is_active());
}

#[test]
fn the_filters_sheet_is_not_available_in_large_mode() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = crate::support::large_document(&directory);
    assert!(!document.can_filter());
    let mut fixture = FiltersFixture::new(document);
    fixture.run();

    assert!(
        fixture
            .query_by_label(tr(EN, "Not available in large file mode."))
            .is_some()
    );
    assert!(
        fixture
            .query_by_role_and_label(Role::Button, tr(EN, "Add Criterion"))
            .is_none()
    );
    assert!(
        fixture
            .query_by_role_and_label(Role::Button, tr(EN, "Apply"))
            .is_none()
    );
}

// -- Settings sheet -------------------------------------------------------

#[test]
fn the_settings_sheet_shows_every_section() {
    let mut fixture = SettingsFixture::new(Settings::default());
    fixture.run();

    for label in ["General", "New Document", "Grid", "Large Files"] {
        assert!(
            fixture.query_by_label(tr(EN, label)).is_some(),
            "missing {label} section"
        );
    }
    assert!(
        fixture
            .query_by_role_and_label(Role::Button, tr(EN, "About"))
            .is_some()
    );
    assert_eq!(
        fixture
            .get_by_role_and_label(Role::ComboBox, "Row Height")
            .value()
            .as_deref(),
        Some("Default")
    );

    fixture.click(tr(EN, "Done"));
    assert!(fixture.closed.get());
}

#[test]
fn changing_the_language_applies_immediately_without_a_restart_hint() {
    let mut fixture = SettingsFixture::new(Settings::default());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::ComboBox, tr(EN, "Language"))
        .click();
    fixture.run();
    fixture.click("Français");

    assert_eq!(fixture.settings.borrow().app_language, Language::Fr);
    assert!(
        fixture
            .query_by_label(tr(Language::Fr, "Settings"))
            .is_some(),
        "the sheet renders in the new language on the next frame"
    );
    assert!(
        fixture
            .query_by_label(tr(EN, "Open TabTool again to apply it."))
            .is_none()
    );
    assert!(
        fixture
            .query_by_label(tr(EN, "Takes effect after restart."))
            .is_none()
    );
}

#[test]
fn settings_edit_the_same_fields_as_desktop() {
    let mut fixture = SettingsFixture::new(Settings::default());
    fixture.run();

    fixture
        .get_by_role_and_label(Role::ComboBox, tr(EN, "Separator"))
        .click();
    fixture.run();
    fixture.click("Semicolon (;)");
    assert_eq!(fixture.settings.borrow().new_document_separator, ";");

    fixture
        .get_by_role_and_label(Role::ComboBox, tr(EN, "Row Height"))
        .click();
    fixture.run();
    fixture.click("Comfortable");
    assert_eq!(fixture.settings.borrow().grid_row_height, 56.0);

    fixture
        .get_by_role_and_label(Role::CheckBox, tr(EN, "Alternating Row Colors"))
        .click();
    fixture.run();
    assert!(fixture.settings.borrow().grid_alternating_rows);

    fixture
        .get_by_role_and_label(Role::ComboBox, tr(EN, "Large File Threshold"))
        .click();
    fixture.run();
    fixture.click("250 MiB");
    assert_eq!(fixture.settings.borrow().large_file_threshold_mib, 250);
}

#[test]
fn the_settings_about_row_opens_the_about_sheet() {
    let mut fixture = SettingsFixture::new(Settings::default());
    fixture.run();

    fixture.click_button(tr(EN, "About"));
    assert!(fixture.about.get());
}

// -- About sheet ----------------------------------------------------------

#[test]
fn the_about_sheet_shows_the_version_website_and_license() {
    let mut fixture = AboutFixture::new();
    fixture.run();

    assert!(
        fixture
            .query_by_label(&tr(EN, "Version %@").replace("%@", "0.1.0"))
            .is_some()
    );
    assert!(
        fixture
            .query_by_label(tr(EN, "tabtool.shurco.app"))
            .is_some()
    );
    assert!(
        fixture
            .query_by_role_and_label(Role::Button, tr(EN, "Rate TabTool"))
            .is_some()
    );
    assert!(fixture.query_by_label(tr(EN, "MIT License")).is_some());
    assert!(
        fixture
            .query_by_label(tr(
                EN,
                "Free and open source. Copyright © 2026 Dmitry Shurco."
            ))
            .is_some()
    );

    fixture.click(tr(EN, "Done"));
    assert!(fixture.closed.get());
}

#[test]
fn the_about_sheet_reports_the_website_and_rate_links() {
    let mut fixture = AboutFixture::new();
    fixture.run();

    fixture.click(tr(EN, "tabtool.shurco.app"));
    assert_eq!(fixture.action.get(), Some(AboutAction::Website));

    fixture.action.set(None);
    fixture.click_button(tr(EN, "Rate TabTool"));
    assert_eq!(fixture.action.get(), Some(AboutAction::Rate));
}

// -- Export sheet ---------------------------------------------------------

#[test]
fn the_export_sheet_shows_every_control_and_defaults_to_csv() {
    let mut fixture = ExportFixture::new(small_document());
    fixture.run();

    for label in ["Encoding", "Separator", "Quote", "Decimal"] {
        assert!(
            fixture
                .query_by_role_and_label(Role::ComboBox, label)
                .is_some(),
            "missing {label} picker"
        );
    }
    assert!(
        fixture
            .query_by_role_and_label(Role::CheckBox, tr(EN, "Write Header Row"))
            .is_some()
    );
    assert!(
        fixture
            .query_by_label(tr(EN, "The header is written only to the exported file."))
            .is_some()
    );

    fixture.click_button(tr(EN, "Export"));
    let calls = fixture.files.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "Untitled-export.csv");
    let text = String::from_utf8(calls[0].bytes.clone()).expect("UTF-8");
    assert!(text.contains("Name"));
    assert!(text.contains("Alice"));
    assert!(fixture.closed.get());
    assert_eq!(
        fixture.document.borrow().configuration().column_separator,
        ","
    );
}

#[test]
fn exporting_writes_a_converted_copy_and_leaves_the_document_unchanged() {
    let mut fixture = ExportFixture::new(small_document());
    fixture.run();

    fixture.choose("Separator", "Semicolon (;)");
    fixture
        .get_by_role_and_label(Role::CheckBox, tr(EN, "Write Header Row"))
        .click();
    fixture.run();

    fixture.click_button(tr(EN, "Export"));
    let calls = fixture.files.calls();
    assert_eq!(calls.len(), 1);
    let text = String::from_utf8(calls[0].bytes.clone()).expect("UTF-8");
    assert!(text.contains("Alice"));
    assert!(text.contains(';'));
    assert!(!text.contains("Name"), "the header toggle is local");
    assert!(!calls[0].source.exists(), "the temporary export is removed");

    let document = fixture.document.borrow();
    assert_eq!(document.configuration().column_separator, ",");
    assert!(document.configuration().first_row_as_header);
    assert_eq!(
        document.table().expect("the table stays").cell(0, 0),
        "Alice"
    );
}

#[test]
fn an_ambiguous_export_asks_before_writing() {
    let mut fixture = ExportFixture::new(multiline_document());
    fixture.run();

    fixture.choose("Quote", "None");
    fixture.click_button(tr(EN, "Export"));
    assert!(fixture.files.calls().is_empty());
    assert!(fixture.query_by_label(tr(EN, "Export Anyway?")).is_some());

    fixture.click_button(tr(EN, "Export Anyway"));
    let calls = fixture.files.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "Untitled-export.csv");
    assert!(fixture.closed.get());
}

#[test]
fn an_unencodable_export_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = file_document(
        &directory,
        "cyrillic.csv",
        "Name,Note\nAlice,Привет\n".as_bytes(),
        header_configuration(),
    );
    let mut fixture = ExportFixture::new(document);
    fixture.run();

    fixture.choose("Encoding", "Western (Windows Latin 1)");
    fixture.click_button(tr(EN, "Export"));

    assert!(fixture.files.calls().is_empty());
    assert!(
        fixture
            .query_by_label(tr(EN, "Export Not Completed"))
            .is_some()
    );
    let message = alert_message(EN, DocumentError::Csv(CsvError::UnencodableText));
    assert!(
        fixture.query_by_label(&message).is_some(),
        "the refusal explains the encoding problem"
    );
    assert!(!fixture.closed.get(), "the sheet stays open");
}

#[test]
fn the_export_sheet_exports_only_the_visible_rows() {
    let mut document = small_document();
    let column = document.row_source().expect("a source").column_ids()[0];
    let criterion = &mut document.row_filter_mut().criteria[0];
    criterion.column_id = Some(column);
    criterion.op = FilterOperator::Equals;
    criterion.value = "Alice".to_owned();
    document.apply_all_filters();
    assert_eq!(document.row_filter().visible_rows, Some(vec![0]));

    let mut fixture = ExportFixture::new(document);
    fixture.run();
    fixture.click_button(tr(EN, "Export"));

    let calls = fixture.files.calls();
    let text = String::from_utf8(calls[0].bytes.clone()).expect("UTF-8");
    assert!(text.contains("Alice"));
    assert!(!text.contains("Bob"));
    assert!(!text.contains("Carol"));
}

#[test]
fn a_pending_export_keeps_its_temporary_file_until_the_copy_finishes() {
    let mut fixture = ExportFixture::new(small_document());
    fixture.files.defer_once();
    fixture.run();

    fixture.click_button(tr(EN, "Export"));
    assert_eq!(fixture.files.calls().len(), 1);
    assert!(fixture.sheet.borrow().is_pending());
    assert!(!fixture.closed.get());
    let source = fixture.files.calls()[0].source.clone();
    assert!(source.exists(), "the picker still needs the file");

    fixture.files.finish_pending();
    fixture.run();

    assert_eq!(fixture.files.calls().len(), 2);
    assert!(fixture.closed.get());
    assert!(!source.exists(), "the export cleans up after the copy");
}

#[test]
fn save_as_copies_a_large_file_and_keeps_the_working_file() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = crate::support::large_document(&directory);
    let path = document
        .file_path()
        .expect("the large fixture is file-backed")
        .to_path_buf();
    let files = ExportFiles::new();
    let mut save = SaveCopy::default();

    assert!(save.start(&document, &files));
    assert_eq!(
        save.take_result(),
        Some(CopyResult::Saved("large.csv".to_owned()))
    );
    let calls = files.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].source, path);
    assert_eq!(calls[0].name, "large.csv");
    assert!(path.exists(), "the working file stays in place");
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn save_as_is_only_for_large_documents() {
    let files = ExportFiles::new();
    let mut save = SaveCopy::default();

    assert!(!save.start(&small_document(), &files));
    assert!(files.calls().is_empty());
    assert_eq!(save.take_result(), None);
}

#[test]
fn save_as_waits_for_an_async_picker() {
    let directory = TempDir::new().expect("a temporary directory");
    let document = crate::support::large_document(&directory);
    let files = ExportFiles::new();
    files.defer_once();
    let mut save = SaveCopy::default();

    assert!(save.start(&document, &files));
    assert!(save.is_pending());
    assert_eq!(save.take_result(), None);

    files.finish_pending();
    save.poll(&files);
    assert!(!save.is_pending());
    assert_eq!(
        save.take_result(),
        Some(CopyResult::Saved("large.csv".to_owned()))
    );
    assert_eq!(files.calls().len(), 2);
}

#[test]
fn the_saved_copy_notice_names_the_file_in_every_language() {
    assert_eq!(
        saved_copy_message(EN, "large.csv"),
        "Saved a copy to large.csv"
    );
    for language in Language::ALL {
        assert!(
            tr_in(language, SAVED_COPY_KEY).is_some(),
            "missing {language:?} translation"
        );
    }
}

#[test]
fn every_sheet_kind_stacks_and_closes_one_at_a_time() {
    let mut nav = NavStack::new();
    nav.open_document(DocumentId::new("/documents/notes.csv"));
    for kind in [
        SheetKind::Format,
        SheetKind::Filters,
        SheetKind::Settings,
        SheetKind::About,
        SheetKind::Export,
    ] {
        nav.open_sheet(kind);
    }

    assert_eq!(nav.sheets().len(), 5);
    for expected in [
        SheetKind::Export,
        SheetKind::About,
        SheetKind::Settings,
        SheetKind::Filters,
        SheetKind::Format,
    ] {
        assert_eq!(nav.sheet(), Some(expected));
        assert_eq!(nav.back(), BackOutcome::ClosedSheet);
    }
    assert_eq!(
        nav.screen(),
        &Screen::Editor(DocumentId::new("/documents/notes.csv"))
    );
    assert_eq!(nav.back(), BackOutcome::ClosedDocument);
}

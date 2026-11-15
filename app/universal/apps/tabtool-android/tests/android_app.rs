//! The Android app lifecycle: navigation, intents, autosave, background,
//! process death, sharing and sheet wiring.
//!
//! The platform is a fake over a temporary directory, and the app's clock is
//! injected, so no test sleeps except the one that observes a real large save.

use core::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use eframe::egui;
use main::app::{
    AndroidApp, BackResult, Host, Notice, PromptChoice, export_outcome, text_input_rect,
};
use main::intents::{IntentEvent, StagedIntent};
use tabtool_document::AUTOSAVE_DELAY;
use tabtool_i18n::{ERROR_OPEN, ERROR_SAVE};
use tabtool_platform::CopyOutcome;
use tabtool_settings::Settings;
use tabtool_ui::grid::Clipboard as GridClipboard;
use tabtool_ui::mobile::{
    DocumentFiles, ExportOutcome, LibraryClock, LibraryError, MobileEditorResponse,
    MobileToolbarResponse, OverflowItem, Screen, SheetKind, TouchTextInput, TouchTextInputOutcome,
    TouchTextInputRequest,
};
use tempfile::TempDir;

// -- Fake platform ------------------------------------------------------------

struct Fake {
    directory: PathBuf,
    settings: RefCell<Settings>,
    settings_writes: Cell<usize>,
    languages: RefCell<Vec<String>>,
    locale: String,
    clock: LibraryClock,
    safe_area: [f32; 4],
    keyboard: Cell<f32>,
    back: Cell<bool>,
    background: Cell<bool>,
    intents: RefCell<VecDeque<IntentEvent>>,
    errors: RefCell<VecDeque<LibraryError>>,
    export_outcomes: RefCell<VecDeque<ExportOutcome>>,
    export_pending: Cell<bool>,
    exports: RefCell<Vec<(PathBuf, String)>>,
    shared: RefCell<Vec<PathBuf>>,
    urls: RefCell<Vec<String>>,
    finished: Cell<usize>,
    background_finishes: Cell<usize>,
    dark_mode: Cell<Option<bool>>,
}

impl Fake {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            directory: std::env::temp_dir(),
            settings: RefCell::new(Settings::default()),
            settings_writes: Cell::new(0),
            languages: RefCell::new(vec!["en-US".to_owned()]),
            locale: "en_US".to_owned(),
            clock: LibraryClock::new(0, false),
            safe_area: [0.0; 4],
            keyboard: Cell::new(0.0),
            back: Cell::new(false),
            background: Cell::new(false),
            intents: RefCell::new(VecDeque::new()),
            errors: RefCell::new(VecDeque::new()),
            export_outcomes: RefCell::new(VecDeque::new()),
            export_pending: Cell::new(false),
            exports: RefCell::new(Vec::new()),
            shared: RefCell::new(Vec::new()),
            urls: RefCell::new(Vec::new()),
            finished: Cell::new(0),
            background_finishes: Cell::new(0),
            dark_mode: Cell::new(None),
        })
    }

    fn settings(&self, settings: Settings) {
        *self.settings.borrow_mut() = settings;
    }

    fn error(&self, error: LibraryError) {
        self.errors.borrow_mut().push_back(error);
    }

    fn intent(&self, event: IntentEvent) {
        self.intents.borrow_mut().push_back(event);
    }

    fn export(&self, outcome: ExportOutcome) {
        self.export_outcomes.borrow_mut().push_back(outcome);
    }
}

impl DocumentFiles for Fake {
    fn documents_dir(&self) -> PathBuf {
        self.directory.clone()
    }

    fn list_documents(&self) -> Vec<PathBuf> {
        let mut documents: Vec<PathBuf> = std::fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && tabtool_core::has_table_extension(path))
            .collect();
        documents.sort();
        documents
    }

    fn existing_names(&self) -> Vec<String> {
        std::fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }

    fn create_document(&self, name: &str, data: &[u8]) -> io::Result<PathBuf> {
        let path = self.directory.join(name);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(data)?;
        Ok(path)
    }

    fn import_from_picker(&self) -> Option<PathBuf> {
        None
    }

    fn export_copy(&self, source: &Path, suggested_name: &str) -> ExportOutcome {
        self.exports
            .borrow_mut()
            .push((source.to_path_buf(), suggested_name.to_owned()));
        self.export_outcomes
            .borrow_mut()
            .pop_front()
            .unwrap_or(ExportOutcome::Cancelled)
    }

    fn export_in_progress(&self) -> bool {
        self.export_pending.get()
    }

    fn delete(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn take_error(&self) -> Option<LibraryError> {
        self.errors.borrow_mut().pop_front()
    }
}

impl Host for Fake {
    fn safe_area(&self) -> [f32; 4] {
        self.safe_area
    }

    fn keyboard_height(&self) -> f32 {
        self.keyboard.get()
    }

    fn take_back(&self) -> bool {
        self.back.take()
    }

    fn share(&self, path: &Path) {
        self.shared.borrow_mut().push(path.to_path_buf());
    }

    fn finish_activity(&self) {
        self.finished.set(self.finished.get() + 1);
    }

    fn languages(&self) -> Vec<String> {
        self.languages.borrow().clone()
    }

    fn dark_mode(&self) -> Option<bool> {
        self.dark_mode.get()
    }

    fn locale(&self) -> String {
        self.locale.clone()
    }

    fn clock(&self) -> LibraryClock {
        self.clock
    }

    fn settings_path(&self) -> Option<PathBuf> {
        Some(self.directory.join("settings.json"))
    }

    fn load_settings(&self) -> Settings {
        self.settings.borrow().clone()
    }

    fn save_settings(&self, settings: &Settings) {
        self.settings_writes.set(self.settings_writes.get() + 1);
        *self.settings.borrow_mut() = settings.clone();
    }

    fn state_path(&self) -> Option<PathBuf> {
        Some(self.directory.join("app-state.json"))
    }

    fn take_background_event(&self) -> bool {
        self.background.take()
    }

    fn background_save_finished(&self) {
        self.background_finishes
            .set(self.background_finishes.get() + 1);
    }

    fn take_intent(&self) -> Option<IntentEvent> {
        self.intents.borrow_mut().pop_front()
    }

    fn open_url(&self, url: &str) {
        self.urls.borrow_mut().push(url.to_owned());
    }
}

use std::io::Write as _;

#[derive(Default)]
struct TextState {
    request: Option<TouchTextInputRequest>,
    pending: Option<TouchTextInputOutcome>,
    draft: Option<String>,
    commits: usize,
}

#[derive(Clone, Default)]
struct FakeText {
    state: Rc<RefCell<TextState>>,
}

impl TouchTextInput for FakeText {
    fn open(&mut self, request: TouchTextInputRequest) {
        let mut state = self.state.borrow_mut();
        state.request = Some(request);
        state.pending = None;
    }

    fn close(&mut self) {
        let mut state = self.state.borrow_mut();
        state.request = None;
        state.pending = None;
    }

    fn commit(&mut self) {
        let mut state = self.state.borrow_mut();
        if let Some(request) = state.request.take() {
            let text = state.draft.take().unwrap_or(request.text);
            state.pending = Some(TouchTextInputOutcome::Committed(text));
            state.commits += 1;
        }
    }

    fn move_to(&mut self, rect: egui::Rect) {
        if let Some(request) = &mut self.state.borrow_mut().request {
            request.rect = rect;
        }
    }

    fn poll(&mut self) -> Option<TouchTextInputOutcome> {
        self.state.borrow_mut().pending.take()
    }

    fn is_active(&self) -> bool {
        self.state.borrow().request.is_some()
    }
}

impl FakeText {
    fn finish(&self, outcome: TouchTextInputOutcome) {
        let mut state = self.state.borrow_mut();
        state.request = None;
        state.pending = Some(outcome);
    }
}

#[derive(Clone, Default)]
struct FakeClipboard {
    text: Rc<RefCell<Option<String>>>,
}

impl GridClipboard for FakeClipboard {
    fn copy(&mut self, text: &str) {
        *self.text.borrow_mut() = Some(text.to_owned());
    }

    fn paste(&mut self) -> Option<String> {
        self.text.borrow().clone()
    }
}

// -- Fixture ------------------------------------------------------------------

struct Fixture {
    directory: TempDir,
    fake: Rc<Fake>,
    input: FakeText,
    clipboard: FakeClipboard,
}

impl Fixture {
    fn new() -> Self {
        let directory = TempDir::new().expect("a temporary directory");
        let mut fake = Fake::new();
        Rc::get_mut(&mut fake)
            .expect("the fake is unique")
            .directory = directory.path().to_path_buf();
        Self {
            directory,
            fake,
            input: FakeText::default(),
            clipboard: FakeClipboard::default(),
        }
    }

    fn app(&self) -> AndroidApp {
        AndroidApp::new(
            self.fake.clone(),
            Box::new(self.input.clone()),
            Box::new(self.clipboard.clone()),
        )
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).expect("the fixture file is written");
        path
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.path(name)).expect("the fixture file is readable")
    }
}

fn open_document(app: &mut AndroidApp, path: &Path) {
    assert!(app.open_document(path), "the fixture opens");
}

fn edit_cell(app: &mut AndroidApp, text: &str) {
    let editor = app.editor_mut().expect("an open editor");
    editor.document_mut().set_cell(0, 0, text);
}

fn wait_for_large_index(app: &mut AndroidApp) {
    let document = app
        .editor_mut()
        .expect("an open large document")
        .document_mut();
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
}

fn large_settings(fixture: &Fixture) {
    fixture.fake.settings(Settings {
        large_file_threshold_mib: 0,
        ..Settings::default()
    });
}

// -- Navigation and back ------------------------------------------------------

#[test]
fn back_pops_sheets_then_the_document_then_exits() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    app.open_sheet(SheetKind::Filters);

    assert_eq!(app.back(), BackResult::ClosedSheet);
    assert!(matches!(app.screen(), Screen::Editor(_)));
    assert_eq!(app.back(), BackResult::ClosedDocument);
    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(app.back(), BackResult::Exit);
    assert_eq!(fixture.fake.finished.get(), 1);
}

#[test]
fn a_clean_editor_closes_without_a_prompt() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);

    assert_eq!(app.back(), BackResult::ClosedDocument);
    assert!(app.prompt().is_none());
    assert!(app.editor().is_none());
}

#[test]
fn a_dirty_small_document_is_saved_before_back_closes_it() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "edited");

    assert_eq!(app.back(), BackResult::ClosedDocument);
    assert!(fixture.read("a.csv").contains("edited"));
    assert!(app.prompt().is_none());
}

#[test]
fn a_dirty_large_document_prompts_and_discard_keeps_the_file() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    edit_cell(&mut app, "edited");

    assert_eq!(app.back(), BackResult::Prompt);
    let prompt = app.prompt().expect("the prompt shows");
    assert_eq!(prompt.name, "large.csv");
    assert!(matches!(app.screen(), Screen::Editor(_)));

    app.answer_prompt(PromptChoice::DontSave);
    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(fixture.read("large.csv"), "a,b\n1,2\n");
}

#[test]
fn a_dirty_large_document_saves_when_the_prompt_asks() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    edit_cell(&mut app, "edited");

    assert_eq!(app.back(), BackResult::Prompt);
    app.answer_prompt(PromptChoice::Save);

    for _ in 0..600 {
        app.poll(Instant::now());
        if matches!(app.screen(), Screen::Library) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(app.screen(), Screen::Library));
    assert!(fixture.read("large.csv").contains("edited"));
}

#[test]
fn cancelling_the_prompt_keeps_the_editor_open() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    edit_cell(&mut app, "edited");

    assert_eq!(app.back(), BackResult::Prompt);
    app.answer_prompt(PromptChoice::Cancel);
    assert!(app.prompt().is_none());
    assert!(matches!(app.screen(), Screen::Editor(_)));
}

// -- Autosave ----------------------------------------------------------------

#[test]
fn a_small_document_autosaves_after_the_debounce() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "edited");
    let start = Instant::now();

    app.poll(start);
    app.poll(start + AUTOSAVE_DELAY - Duration::from_millis(1));
    assert!(!fixture.read("a.csv").contains("edited"));

    app.poll(start + AUTOSAVE_DELAY);
    assert!(fixture.read("a.csv").contains("edited"));
    assert!(!app.editor().expect("editor").document().is_dirty());
}

#[test]
fn a_large_document_is_never_autosaved() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    edit_cell(&mut app, "edited");

    app.poll(Instant::now() + Duration::from_secs(60));

    assert_eq!(fixture.read("large.csv"), "a,b\n1,2\n");
    assert!(app.editor().expect("editor").document().is_dirty());
}

#[test]
fn an_untitled_document_is_never_autosaved() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    let editor = app.editor_mut().expect("an open editor");
    editor
        .document_mut()
        .set_file_path(None)
        .expect("the document becomes untitled");
    editor.document_mut().set_cell(0, 0, "edited");

    app.poll(Instant::now() + Duration::from_secs(60));

    assert_eq!(fixture.read("a.csv"), "a,b\n1,2\n");
    assert!(app.editor().expect("editor").document().is_dirty());
}

#[test]
fn back_defers_while_a_native_edit_is_open() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    let mut input = fixture.input.clone();
    input.open(TouchTextInputRequest {
        rect: egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, 44.0)),
        text: "draft".to_owned(),
        multiline: false,
    });

    assert_eq!(app.back(), BackResult::Deferred);
    assert!(matches!(app.screen(), Screen::Editor(_)));

    input.finish(TouchTextInputOutcome::Committed("draft".to_owned()));
    app.poll(Instant::now());
    assert!(matches!(app.screen(), Screen::Library));
}

// -- Background ---------------------------------------------------------------

#[test]
fn background_saves_a_dirty_small_document() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "edited");

    fixture.fake.background.set(true);
    app.poll(Instant::now());

    assert!(fixture.read("a.csv").contains("edited"));
    assert!(!app.editor().expect("editor").document().is_dirty());
}

#[test]
fn background_waits_for_an_open_native_edit() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "edited");
    let mut input = fixture.input.clone();
    input.open(TouchTextInputRequest {
        rect: egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, 44.0)),
        text: "draft".to_owned(),
        multiline: false,
    });

    fixture.fake.background.set(true);
    app.poll(Instant::now());

    assert!(
        !fixture.read("a.csv").contains("edited"),
        "no save while editing"
    );
    assert_eq!(fixture.fake.background_finishes.get(), 0);

    input.close();
    app.poll(Instant::now());
    assert!(fixture.read("a.csv").contains("edited"));
    assert_eq!(fixture.fake.background_finishes.get(), 1);
}

#[test]
fn background_leaves_a_large_document_dirty() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    edit_cell(&mut app, "edited");

    fixture.fake.background.set(true);
    app.poll(Instant::now());

    assert_eq!(fixture.read("large.csv"), "a,b\n1,2\n");
    assert!(app.editor().expect("editor").document().is_dirty());
}

// -- Intents ------------------------------------------------------------------

#[test]
fn an_intent_copy_is_adopted_under_a_free_name_and_opened() {
    let fixture = Fixture::new();
    fixture.write("data.csv", "old\n1\n");
    fixture.write(".incoming-1.csv", "x,y\n3,4\n");
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-1.csv".to_owned(),
        display_name: "data.csv".to_owned(),
    }));
    let mut app = fixture.app();

    app.poll(Instant::now());

    let Screen::Editor(document) = app.screen() else {
        panic!("the imported document opens: {:?}", app.screen());
    };
    assert_eq!(
        document.path().file_name().and_then(|name| name.to_str()),
        Some("data 2.csv")
    );
    assert_eq!(fixture.read("data 2.csv"), "x,y\n3,4\n");
    assert_eq!(fixture.read("data.csv"), "old\n1\n");
    assert!(!fixture.path(".incoming-1.csv").exists());
}

#[test]
fn a_failed_intent_shows_a_banner_and_stays_in_the_library() {
    let fixture = Fixture::new();
    fixture
        .fake
        .intent(IntentEvent::Failed("access denied".to_owned()));
    let mut app = fixture.app();

    app.poll(Instant::now());

    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(app.notice(), Some(&Notice::Error(ERROR_OPEN)));
}

#[test]
fn a_malformed_intent_staging_file_is_removed_and_reported() {
    let fixture = Fixture::new();
    fixture.write(".incoming-2.bin", "binary");
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-2.bin".to_owned(),
        display_name: "data.bin".to_owned(),
    }));
    let mut app = fixture.app();

    app.poll(Instant::now());

    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(app.notice(), Some(&Notice::Error(ERROR_OPEN)));
    assert!(!fixture.path(".incoming-2.bin").exists());
}

// -- Process death ------------------------------------------------------------

#[test]
fn the_editor_and_sheet_are_restored_after_process_death() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    {
        let mut app = fixture.app();
        open_document(&mut app, &path);
        app.open_sheet(SheetKind::Filters);
    }

    let restored = fixture.app();

    let Screen::Editor(document) = restored.screen() else {
        panic!("the editor is restored: {:?}", restored.screen());
    };
    assert_eq!(document.path(), path);
    assert_eq!(restored.nav().sheet(), Some(SheetKind::Filters));
    assert_eq!(restored.editor().expect("editor").title(), "a.csv");
}

#[test]
fn the_full_sheet_stack_is_restored_after_process_death() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    {
        let mut app = fixture.app();
        open_document(&mut app, &path);
        app.open_sheet(SheetKind::Settings);
        app.open_sheet(SheetKind::About);
    }

    let mut restored = fixture.app();

    assert_eq!(
        restored.nav().sheets(),
        &[SheetKind::Settings, SheetKind::About]
    );
    assert_eq!(restored.back(), BackResult::ClosedSheet);
    assert_eq!(restored.nav().sheet(), Some(SheetKind::Settings));
}

#[test]
fn a_missing_restored_document_returns_to_the_library_with_a_notice() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    {
        let mut app = fixture.app();
        open_document(&mut app, &path);
    }
    std::fs::remove_file(&path).expect("the document is deleted");

    let restored = fixture.app();

    assert!(matches!(restored.screen(), Screen::Library));
    assert_eq!(restored.notice(), Some(&Notice::Error(ERROR_OPEN)));
}

// -- Sheets, share and export -------------------------------------------------

#[test]
fn the_editor_toolbar_opens_the_filter_and_export_sheets() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            filter: true,
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });
    assert_eq!(app.nav().sheet(), Some(SheetKind::Filters));
    assert_eq!(app.back(), BackResult::ClosedSheet);

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            export: true,
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });
    assert_eq!(app.nav().sheet(), Some(SheetKind::Export));
    assert_eq!(app.back(), BackResult::ClosedSheet);
}

#[test]
fn the_overflow_opens_settings_and_about_even_from_the_library() {
    let fixture = Fixture::new();
    let mut app = fixture.app();

    app.open_sheet(SheetKind::Settings);
    assert_eq!(app.nav().sheet(), Some(SheetKind::Settings));
    app.open_sheet(SheetKind::About);
    assert_eq!(app.nav().sheet(), Some(SheetKind::About));
    assert_eq!(app.back(), BackResult::ClosedSheet);
    assert_eq!(app.nav().sheet(), Some(SheetKind::Settings));
    assert_eq!(app.back(), BackResult::ClosedSheet);
    assert_eq!(app.back(), BackResult::Exit);
}

#[test]
fn sharing_hands_the_current_document_to_the_platform() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);

    app.share_current_document();

    assert_eq!(
        fixture.fake.shared.borrow().as_slice(),
        std::slice::from_ref(&path)
    );
}

#[test]
fn the_overflow_share_saves_a_dirty_document_before_sharing() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "saved first");

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            overflow: Some(OverflowItem::Share),
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });

    assert!(fixture.read("a.csv").contains("saved first"));
    assert_eq!(
        fixture.fake.shared.borrow().as_slice(),
        std::slice::from_ref(&path)
    );
}

#[test]
fn two_quick_intents_are_both_adopted() {
    let fixture = Fixture::new();
    fixture.write(".incoming-1.csv", "one,1\n");
    fixture.write(".incoming-2.csv", "two,2\n");
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-1.csv".to_owned(),
        display_name: "one.csv".to_owned(),
    }));
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-2.csv".to_owned(),
        display_name: "two.csv".to_owned(),
    }));
    let mut app = fixture.app();

    app.poll(Instant::now());

    assert!(fixture.path("one.csv").exists());
    assert!(fixture.path("two.csv").exists());
    assert!(!fixture.path(".incoming-1.csv").exists());
    assert!(!fixture.path(".incoming-2.csv").exists());
    let Screen::Editor(document) = app.screen() else {
        panic!("the last imported document opens: {:?}", app.screen());
    };
    assert_eq!(
        document.path().file_name().and_then(|name| name.to_str()),
        Some("two.csv")
    );
}

#[test]
fn startup_removes_an_abandoned_staging_file() {
    let fixture = Fixture::new();
    let old = fixture.write(".incoming-9.csv", "old");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&old)
        .expect("the staging file is writable");
    file.set_times(
        std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(1)),
    )
    .expect("the old time is set");

    let _app = fixture.app();

    assert!(!old.exists());
}

#[test]
fn save_as_copies_the_large_document_and_reports_success() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    fixture.fake.export(ExportOutcome::Copied);

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            overflow: Some(OverflowItem::SaveAs),
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });
    app.poll(Instant::now());

    assert!(
        matches!(app.notice(), Some(Notice::Message(message)) if message.contains("large.csv"))
    );
    let exports = fixture.fake.exports.borrow();
    assert_eq!(exports.len(), 1);
    assert_eq!(exports[0].0, path);
}

#[test]
fn a_cancelled_export_does_not_show_a_failure() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    fixture.fake.export(ExportOutcome::Cancelled);

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            overflow: Some(OverflowItem::SaveAs),
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });
    app.poll(Instant::now());

    assert!(app.notice().is_none());
}

#[test]
fn a_failed_export_shows_the_save_failure_banner() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);
    fixture.fake.export(ExportOutcome::Failed);

    app.apply_editor_response(&MobileEditorResponse {
        toolbar: MobileToolbarResponse {
            overflow: Some(OverflowItem::SaveAs),
            ..MobileToolbarResponse::default()
        },
        ..MobileEditorResponse::default()
    });
    app.poll(Instant::now());

    assert_eq!(app.notice(), Some(&Notice::Error(ERROR_SAVE)));
}

#[test]
fn platform_errors_become_localized_banners() {
    let fixture = Fixture::new();
    fixture.fake.error(LibraryError::new(ERROR_OPEN, "detail"));
    let mut app = fixture.app();

    app.poll(Instant::now());
    assert_eq!(app.notice(), Some(&Notice::Error(ERROR_OPEN)));
}

// -- Pure geometry and outcome mapping ---------------------------------------

#[test]
fn native_text_rects_are_shifted_by_the_content_origin() {
    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 44.0));

    assert_eq!(
        text_input_rect(rect, [8.0, 24.0, 0.0, 0.0]),
        [18.0, 44.0, 100.0, 44.0]
    );
}

#[test]
fn the_export_outcome_distinguishes_pending_cancel_and_failure() {
    assert_eq!(export_outcome(None, true), ExportOutcome::Cancelled);
    assert_eq!(export_outcome(None, false), ExportOutcome::Failed);
    assert_eq!(
        export_outcome(Some(CopyOutcome::Cancelled), false),
        ExportOutcome::Cancelled
    );
    assert_eq!(
        export_outcome(Some(CopyOutcome::Failed), false),
        ExportOutcome::Failed
    );
    assert_eq!(
        export_outcome(Some(CopyOutcome::Copied), false),
        ExportOutcome::Copied
    );
}

#[test]
fn the_interface_language_follows_the_setting() {
    let fixture = Fixture::new();
    *fixture.fake.languages.borrow_mut() = vec!["de-DE".to_owned()];
    fixture.fake.settings(Settings {
        app_language: tabtool_i18n::Language::Fr,
        ..Settings::default()
    });
    let app = fixture.app();

    assert_eq!(app.language(), tabtool_i18n::Language::Fr);
}

#[test]
fn the_system_locale_is_parsed_with_an_english_fallback() {
    assert_eq!(
        main::library::parse_locale("de-DE").id.language.as_str(),
        "de"
    );
    assert_eq!(
        main::library::parse_locale("en_US")
            .id
            .region
            .map(|region| region.as_str().to_owned()),
        Some("US".to_owned()),
        "the platform reports underscore tags"
    );
    assert_eq!(
        main::library::parse_locale("not a locale")
            .id
            .language
            .as_str(),
        "en"
    );
}

// -- End-to-end scenarios -----------------------------------------------------
//
// These chain the per-behavior tests above into the journeys the emulator
// checklist walks: open, edit, autosave, export, share and back; two intents
// that collide on a name; process death while a sheet is open; and the large
// document's read-only structure and prompt on back.

#[test]
fn an_edit_survives_autosave_the_export_sheet_share_and_back() {
    let fixture = Fixture::new();
    let path = fixture.write("Budget.csv", "Name,Amount\nAlice,1\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    edit_cell(&mut app, "42");

    let start = Instant::now();
    app.poll(start);
    app.poll(start + AUTOSAVE_DELAY);
    assert!(fixture.read("Budget.csv").contains("42"));
    assert!(!app.editor().expect("editor").document().is_dirty());

    app.open_sheet(SheetKind::Export);
    assert_eq!(app.nav().sheet(), Some(SheetKind::Export));
    assert_eq!(app.back(), BackResult::ClosedSheet);
    assert_eq!(app.nav().sheet(), None);

    app.share_current_document();
    assert_eq!(
        fixture.fake.shared.borrow().as_slice(),
        std::slice::from_ref(&path)
    );

    assert_eq!(app.back(), BackResult::ClosedDocument);
    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(app.back(), BackResult::Exit);
    assert_eq!(fixture.fake.finished.get(), 1);
}

#[test]
fn two_imports_with_the_same_name_get_distinct_names() {
    let fixture = Fixture::new();
    fixture.write("data.csv", "old\n1\n");
    fixture.write(".incoming-1.csv", "one,1\n");
    fixture.write(".incoming-2.csv", "two,2\n");
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-1.csv".to_owned(),
        display_name: "data.csv".to_owned(),
    }));
    fixture.fake.intent(IntentEvent::Staged(StagedIntent {
        file_name: ".incoming-2.csv".to_owned(),
        display_name: "data.csv".to_owned(),
    }));
    let mut app = fixture.app();

    app.poll(Instant::now());
    let Screen::Editor(opened) = app.screen() else {
        panic!("the last import opens: {:?}", app.screen());
    };
    assert_eq!(
        opened.path().file_name().and_then(|name| name.to_str()),
        Some("data 3.csv")
    );
    assert_eq!(app.back(), BackResult::ClosedDocument);
    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(fixture.read("data.csv"), "old\n1\n");
    assert_eq!(fixture.read("data 2.csv"), "one,1\n");
    assert_eq!(fixture.read("data 3.csv"), "two,2\n");
    assert!(!fixture.path(".incoming-1.csv").exists());
    assert!(!fixture.path(".incoming-2.csv").exists());
}

#[test]
fn process_death_restores_the_document_and_each_sheet_in_order() {
    let fixture = Fixture::new();
    let path = fixture.write("a.csv", "a,b\n1,2\n");
    {
        let mut app = fixture.app();
        open_document(&mut app, &path);
        edit_cell(&mut app, "unsaved");
        app.open_sheet(SheetKind::Settings);
        app.open_sheet(SheetKind::About);
    }

    let mut restored = fixture.app();
    let Screen::Editor(id) = restored.screen() else {
        panic!("the editor is restored: {:?}", restored.screen());
    };
    assert_eq!(id.path(), path);
    assert!(
        !restored.editor().expect("editor").document().is_dirty(),
        "process death keeps the last save"
    );
    assert_eq!(
        restored.nav().sheets(),
        &[SheetKind::Settings, SheetKind::About]
    );
    assert_eq!(restored.back(), BackResult::ClosedSheet);
    assert_eq!(restored.back(), BackResult::ClosedSheet);
    assert_eq!(restored.back(), BackResult::ClosedDocument);
    assert!(matches!(restored.screen(), Screen::Library));
}

#[test]
fn a_large_document_is_read_only_until_it_is_saved_or_discarded() {
    let fixture = Fixture::new();
    large_settings(&fixture);
    let path = fixture.write("large.csv", "a,b\n1,2\n");
    let mut app = fixture.app();
    open_document(&mut app, &path);
    wait_for_large_index(&mut app);

    {
        let document = app.editor().expect("editor").document();
        assert!(document.is_large_mode());
        assert!(!document.can_edit_structure());
        assert!(!document.can_select_all());
    }

    edit_cell(&mut app, "edited");
    app.poll(Instant::now() + Duration::from_secs(60));
    assert_eq!(
        fixture.read("large.csv"),
        "a,b\n1,2\n",
        "a large document is never autosaved"
    );

    assert_eq!(app.back(), BackResult::Prompt);
    app.answer_prompt(PromptChoice::DontSave);
    assert!(matches!(app.screen(), Screen::Library));
    assert_eq!(fixture.read("large.csv"), "a,b\n1,2\n");
}

#[test]
fn the_system_theme_is_applied_and_followed() {
    use eframe::egui::{Context, ThemePreference};

    let fixture = Fixture::new();
    let app = fixture.app();
    let ctx = Context::default();

    for (reported, expected) in [
        (Some(false), ThemePreference::Light),
        (Some(true), ThemePreference::Dark),
    ] {
        fixture.fake.dark_mode.set(reported);
        app.apply_system_theme(&ctx);
        assert_eq!(ctx.options(|options| options.theme_preference), expected);
    }

    fixture.fake.dark_mode.set(None);
    app.apply_system_theme(&ctx);
    assert_eq!(
        ctx.options(|options| options.theme_preference),
        ThemePreference::Dark,
        "a platform without a theme leaves the last one alone"
    );
}

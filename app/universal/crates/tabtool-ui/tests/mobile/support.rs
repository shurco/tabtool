//! Documents, the native text-input fake and the touch editor fixture.

use core::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use egui::accesskit::Role;
use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::Document;
use tabtool_i18n::Language;
use tabtool_ui::grid::{Clipboard, GridAppearance, GridSelection, GridState};
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{
    FoldRegion, MobileEditTarget, MobileEditor, MobileEditorResponse, MobileEditorState,
    MobileMetrics, TouchTextInput, TouchTextInputOutcome, TouchTextInputRequest,
};
use tabtool_ui::{Theme, UiHarness};
use tempfile::TempDir;

/// The fixture configuration: the first row names the columns.
fn header_configuration() -> CsvConfiguration {
    CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    }
}

/// A document with a header and `rows` data rows named `Row 0` onwards.
pub fn document(rows: usize) -> Document {
    let mut text = String::from("Name,Amount\n");
    for row in 0..rows {
        text.push_str(&format!("Row {row},{row}.5\n"));
    }
    let configuration = header_configuration();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(text.as_bytes(), None, Some(configuration), false)
        .expect("the fixture parses");
    document
}

/// A three-row document used by most tests.
pub fn small_document() -> Document {
    let configuration = header_configuration();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"Name,Amount\nAlice,1.5\nBob,20\nCarol,-3.25\n",
            None,
            Some(configuration),
            false,
        )
        .expect("the fixture parses");
    document
}

/// A document whose second cell contains a quoted line break.
pub fn multiline_document() -> Document {
    let configuration = header_configuration();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"Name,Note\nAlice,\"first\nsecond\"\n",
            None,
            Some(configuration),
            false,
        )
        .expect("the quoted cell parses");
    document
}

/// Opens `path` through the large-file path and waits for its index.
pub fn large_document(directory: &TempDir) -> Document {
    let path: std::path::PathBuf = directory.path().join("large.csv");
    std::fs::write(&path, "Name,Amount\nAlice,1\nBob,2\nCarol,3\n")
        .expect("the large fixture is written");
    open_large(Path::new(&path))
}

/// Opens a prepared large file and waits for its index.
pub fn open_large(path: &Path) -> Document {
    let mut document = Document::new(header_configuration());
    document
        .open_large(path, Some(header_configuration()), false, 0)
        .expect("the large fixture opens");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
    document
}

/// A clipboard that holds no text; the editor tests never paste.
#[derive(Default)]
pub struct TestClipboard;

impl Clipboard for TestClipboard {
    fn copy(&mut self, _text: &str) {}

    fn paste(&mut self) -> Option<String> {
        None
    }
}

#[derive(Default)]
struct FakeState {
    request: Option<TouchTextInputRequest>,
    pending: Option<TouchTextInputOutcome>,
    draft: Option<String>,
    placed: Option<egui::Rect>,
    commits: usize,
}

/// A platform text editor fake the tests drive by hand.
#[derive(Clone, Default)]
pub struct FakeTouchInput {
    state: Rc<RefCell<FakeState>>,
}

impl FakeTouchInput {
    /// The request the editor opened, while it is open.
    pub fn request(&self) -> Option<TouchTextInputRequest> {
        self.state.borrow().request.clone()
    }

    /// The text the user typed before the next commit.
    pub fn set_draft(&self, text: &str) {
        self.state.borrow_mut().draft = Some(text.to_owned());
    }

    /// Reports an outcome without going through commit, as the platform does.
    pub fn finish(&self, outcome: TouchTextInputOutcome) {
        let mut state = self.state.borrow_mut();
        state.request = None;
        state.pending = Some(outcome);
    }

    /// The number of commit requests the accessory and taps produced.
    pub fn commits(&self) -> usize {
        self.state.borrow().commits
    }

    /// The rectangle the native editor currently covers, in screen points.
    pub fn placed_rect(&self) -> Option<egui::Rect> {
        self.state.borrow().placed
    }
}

impl TouchTextInput for FakeTouchInput {
    fn open(&mut self, request: TouchTextInputRequest) {
        let mut state = self.state.borrow_mut();
        state.placed = Some(request.rect);
        state.request = Some(request);
        state.pending = None;
    }

    fn close(&mut self) {
        let mut state = self.state.borrow_mut();
        state.request = None;
        state.pending = None;
        state.placed = None;
    }

    fn commit(&mut self) {
        let mut state = self.state.borrow_mut();
        if let Some(request) = state.request.take() {
            let text = state.draft.take().unwrap_or(request.text);
            state.pending = Some(TouchTextInputOutcome::Committed(text));
            state.commits += 1;
        }
        state.placed = None;
    }

    fn move_to(&mut self, rect: egui::Rect) {
        let mut state = self.state.borrow_mut();
        state.placed = Some(rect);
        if let Some(request) = &mut state.request {
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

/// The touch editor rendered by the deterministic kittest harness.
pub struct EditorFixture {
    ui: UiHarness<'static>,
    /// The document the editor edits.
    pub document: Rc<RefCell<Document>>,
    /// The persistent grid state.
    pub state: Rc<RefCell<GridState>>,
    /// The editor's own state.
    pub editor: Rc<RefCell<MobileEditorState>>,
    /// The native text editor the tests drive.
    pub input: FakeTouchInput,
    /// The responses collected since the last take.
    pub response: Rc<RefCell<MobileEditorResponse>>,
    /// The keyboard inset the app reports.
    pub keyboard: Rc<Cell<f32>>,
    /// The active fold, in screen points.
    pub fold: Rc<Cell<Option<FoldRegion>>>,
    _directory: Option<TempDir>,
}

impl EditorFixture {
    /// A phone-sized fixture.
    pub fn phone(document: Document) -> Self {
        Self::new(document, [360.0, 800.0], MobileMetrics::phone())
    }

    /// A fixture at an explicit size.
    pub fn new(document: Document, size: [f32; 2], metrics: MobileMetrics) -> Self {
        let document = Rc::new(RefCell::new(document));
        let state = Rc::new(RefCell::new(GridState::default()));
        let editor = Rc::new(RefCell::new(MobileEditorState::default()));
        let input = FakeTouchInput::default();
        let response = Rc::new(RefCell::new(MobileEditorResponse::default()));
        let keyboard = Rc::new(Cell::new(0.0));
        let fold = Rc::new(Cell::new(None));
        let locale = Locale::try_from_str("en-US").expect("en-US is a valid locale");

        let ui_document = Rc::clone(&document);
        let ui_state = Rc::clone(&state);
        let ui_editor = Rc::clone(&editor);
        let ui_input = input.clone();
        let ui_response = Rc::clone(&response);
        let ui_keyboard = Rc::clone(&keyboard);
        let ui_fold = Rc::clone(&fold);

        let ui = UiHarness::new(Theme::desktop_light(), size, move |ui| {
            let mut document = ui_document.borrow_mut();
            let mut state = ui_state.borrow_mut();
            let mut editor = ui_editor.borrow_mut();
            let mut input = ui_input.clone();
            let mut clipboard = TestClipboard;
            let shown = MobileEditor {
                document: &mut document,
                state: &mut state,
                editor_state: &mut editor,
                text_input: &mut input,
                clipboard: &mut clipboard,
                metrics,
                language: Language::En,
                title: "Budget.csv",
                locale: &locale,
                decimal_separator: ".",
                appearance: GridAppearance::default(),
                keyboard_height: ui_keyboard.get(),
                fold: ui_fold.get(),
            }
            .show(ui);
            let mut response = ui_response.borrow_mut();
            response.toolbar.merge(shown.toolbar);
            if shown.large.is_some() {
                response.large = shown.large;
            }
            response.show_format_bar |= shown.show_format_bar;
        });

        Self {
            ui,
            document,
            state,
            editor,
            input,
            response,
            keyboard,
            fold,
            _directory: None,
        }
    }

    /// Keeps a temporary directory alive with the fixture.
    pub fn keep(mut self, directory: TempDir) -> Self {
        self._directory = Some(directory);
        self
    }

    /// Runs one frame.
    pub fn run(&mut self) {
        self.ui.run();
    }

    /// Sets the keyboard inset the app reports.
    pub fn set_keyboard(&self, height: f32) {
        self.keyboard.set(height);
    }

    /// Sets the active fold.
    pub fn set_fold(&self, fold: Option<FoldRegion>) {
        self.fold.set(fold);
    }

    /// Resizes the rendered window.
    pub fn resize(&mut self, size: [f32; 2]) {
        self.ui.set_size(egui::vec2(size[0], size[1]));
    }

    /// Clicks a data cell and runs a frame.
    pub fn click_cell(&mut self, text: &str) {
        self.ui.get_by_role_and_label(Role::Cell, text).click();
        self.run();
    }

    /// Clicks a column header and runs a frame.
    pub fn click_header(&mut self, text: &str) {
        self.ui
            .get_by_role_and_label(Role::ColumnHeader, text)
            .click();
        self.run();
    }

    /// Clicks a control by its accessible name and runs a frame.
    pub fn click(&mut self, label: &str) {
        self.ui.get_by_label(label).click();
        self.run();
    }

    /// The text of a model cell.
    pub fn cell(&self, row: usize, column: usize) -> String {
        self.document
            .borrow()
            .table()
            .expect("the fixture has a table")
            .cell(row, column)
    }

    /// The current grid selection.
    pub fn selection(&self) -> GridSelection {
        self.state.borrow().selection.clone()
    }

    /// The target the native editor edits.
    pub fn edit_target(&self) -> Option<MobileEditTarget> {
        self.editor
            .borrow()
            .editing
            .as_ref()
            .map(|editing| editing.target)
    }

    /// Takes the responses collected since the last call.
    pub fn take_response(&self) -> MobileEditorResponse {
        std::mem::take(&mut *self.response.borrow_mut())
    }

    /// Whether a node with `label` exists in the current frame.
    pub fn has_label(&self, label: &str) -> bool {
        self.ui.query_by_label(label).is_some()
    }
}

impl core::ops::Deref for EditorFixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for EditorFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

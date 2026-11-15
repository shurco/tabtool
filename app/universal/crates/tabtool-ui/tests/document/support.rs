//! Documents, storage and the kittest fixture for the document chrome tests.

#![allow(dead_code)]

use core::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::Document;
use tabtool_ui::document::{DocumentAction, DocumentUiState, DocumentView};
use tabtool_ui::grid::GridState;
use tabtool_ui::harness::TestClipboard;
use tabtool_ui::harness::kittest::{NodeT, Queryable};
use tabtool_ui::{Theme, UiHarness};

#[allow(unused_imports)] // Each test crate re-exports a different subset.
pub use tabtool_document::test_support::{MemoryStorage, large_ready, loaded, untitled};

/// A document whose initial read failed with `bytes` but whose storage holds
/// valid text, so Detect Automatically can recover it.
pub fn read_failed(_storage: &MemoryStorage, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"\xFF",
            Some(Path::new("/virtual/document.csv")),
            Some(configuration),
            false,
        )
        .expect_err("the invalid bytes fail to decode");
    document
}

/// The chrome and document rendered by the deterministic kittest harness.
pub struct Fixture {
    ui: UiHarness<'static>,
    document: Rc<RefCell<Document>>,
    state: Rc<RefCell<GridState>>,
    ui_state: Rc<RefCell<DocumentUiState>>,
    actions: Rc<RefCell<Vec<DocumentAction>>>,
    storage: Rc<MemoryStorage>,
    /// Whether the chrome is told a modal owns the keyboard.
    input_blocked: Rc<Cell<bool>>,
    /// A large fixture's file lives as long as the harness does.
    _directory: Option<tempfile::TempDir>,
}

impl Fixture {
    /// Builds a fixture at the default window size.
    pub fn new(document: Document, storage: MemoryStorage, ui_state: DocumentUiState) -> Self {
        Self::new_with(document, storage, ui_state, UiHarness::WIDE, true, None)
    }

    /// Builds a fixture at `size`.
    pub fn sized(
        document: Document,
        storage: MemoryStorage,
        ui_state: DocumentUiState,
        size: [f32; 2],
    ) -> Self {
        Self::new_with(document, storage, ui_state, size, true, None)
    }

    /// Builds a fixture with every option the tests vary.
    pub fn new_with(
        document: Document,
        storage: MemoryStorage,
        ui_state: DocumentUiState,
        size: [f32; 2],
        show_format_bar: bool,
        save_error: Option<tabtool_document::DocumentError>,
    ) -> Self {
        let document = Rc::new(RefCell::new(document));
        let state = Rc::new(RefCell::new(GridState::default()));
        let ui_state = Rc::new(RefCell::new(ui_state));
        let actions = Rc::new(RefCell::new(Vec::new()));
        let storage = Rc::new(storage);
        let input_blocked = Rc::new(Cell::new(false));
        let locale = Locale::try_from_str("en").expect("English is a valid locale");

        let ui_document = Rc::clone(&document);
        let ui_state_rc = Rc::clone(&ui_state);
        let ui_actions = Rc::clone(&actions);
        let ui_storage = Rc::clone(&storage);
        let ui_grid = Rc::clone(&state);
        let ui_blocked = Rc::clone(&input_blocked);
        let ui = UiHarness::new(Theme::desktop_light(), size, move |ui| {
            let mut document = ui_document.borrow_mut();
            let mut state = ui_grid.borrow_mut();
            let mut ui_state = ui_state_rc.borrow_mut();
            let mut clipboard = TestClipboard;
            let shown = DocumentView {
                document: &mut document,
                state: &mut state,
                ui_state: &mut ui_state,
                storage: &*ui_storage,
                clipboard: &mut clipboard,
                language: tabtool_i18n::Language::En,
                prefers_chinese: false,
                locale: &locale,
                decimal_separator: ".",
                show_format_bar,
                appearance: tabtool_ui::grid::GridAppearance::default(),
                input_blocked: ui_blocked.get(),
                save_error,
            }
            .show(ui);
            ui_actions.borrow_mut().extend(shown);
        });

        Self {
            ui,
            document,
            state,
            ui_state,
            actions,
            storage,
            input_blocked,
            _directory: None,
        }
    }

    /// Keeps a large fixture's temporary directory alive with the harness.
    pub fn keep(mut self, directory: tempfile::TempDir) -> Self {
        self._directory = Some(directory);
        self
    }

    /// Runs one frame.
    pub fn run(&mut self) {
        self.ui.run();
    }

    /// Runs two frames so a status line drawn before a bottom control updates.
    pub fn settle(&mut self) {
        self.run();
        self.run();
    }

    /// The actions the view produced since the last call.
    pub fn take_actions(&self) -> Vec<DocumentAction> {
        std::mem::take(&mut *self.actions.borrow_mut())
    }

    /// Looks at the document immutably.
    pub fn with_document<T>(&self, f: impl FnOnce(&Document) -> T) -> T {
        f(&self.document.borrow())
    }

    /// Looks at the document mutably.
    pub fn with_document_mut<T>(&self, f: impl FnOnce(&mut Document) -> T) -> T {
        f(&mut self.document.borrow_mut())
    }

    /// Looks at the chrome state.
    pub fn with_ui_state<T>(&self, f: impl FnOnce(&mut DocumentUiState) -> T) -> T {
        f(&mut self.ui_state.borrow_mut())
    }

    /// Looks at the grid state.
    pub fn grid(&self) -> core::cell::Ref<'_, GridState> {
        self.state.borrow()
    }

    /// Changes the grid selection.
    pub fn set_selection(&self, selection: tabtool_ui::grid::GridSelection) {
        self.state.borrow_mut().selection = selection;
    }

    /// Seeds an open cell editor, as if the user double-clicked and typed.
    pub fn edit_cell(&self, row: usize, column: usize, text: &str) {
        let mut state = self.state.borrow_mut();
        state.editing = Some(tabtool_ui::grid::CellEditor::new(row, column, text));
        state.focus = Some(tabtool_ui::grid::GridFocus::new(row, column));
    }

    /// Tells the chrome that a modal owns the keyboard.
    pub fn set_input_blocked(&self, blocked: bool) {
        self.input_blocked.set(blocked);
    }

    /// The storage behind the fixture.
    pub fn storage(&self) -> &MemoryStorage {
        &self.storage
    }

    /// Takes a snapshot.
    #[cfg(target_os = "macos")]
    pub fn snapshot(&mut self, name: impl Into<String>) {
        self.ui.snapshot(name);
    }
}

impl core::ops::Deref for Fixture {
    type Target = UiHarness<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl core::ops::DerefMut for Fixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}

/// Whether the node labelled `label` is disabled.
pub fn is_disabled(ui: &UiHarness<'_>, label: &str) -> bool {
    let node = ui.get_by_label(label);
    node.accesskit_node().is_disabled()
}

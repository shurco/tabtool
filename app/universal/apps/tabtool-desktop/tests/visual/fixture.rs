//! The deterministic document harness and shared snapshot data.

use std::cell::RefCell;
use std::rc::Rc;

use tabtool_core::{CsvConfiguration, Locale};
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_ui::UiHarness;
use tabtool_ui::document::{DocumentUiState, DocumentView};
use tabtool_ui::grid::{Clipboard, GridAppearance, GridState};
use tabtool_ui::harness::SnapshotResults;

use crate::support::{MemoryStorage, loaded};

/// The CSV used by the grid and chrome snapshots.
pub const CSV: &[u8] = b"name,amount,note\nApple,1.5,Fruit\nBanana,0.5,Fruit\nCarrot,2,Veg\n";

/// A clipboard that never holds text; the snapshots never paste.
struct TestClipboard;

impl Clipboard for TestClipboard {
    fn copy(&mut self, _text: &str) {}

    fn paste(&mut self) -> Option<String> {
        None
    }
}

/// A document chrome rendered by the deterministic harness.
pub struct DocumentFixture {
    ui: UiHarness<'static>,
    document: Rc<RefCell<Document>>,
    state: Rc<RefCell<GridState>>,
    directory: Option<tempfile::TempDir>,
}

impl DocumentFixture {
    /// Builds a fixture with its own storage for detection.
    pub fn new(
        theme: tabtool_ui::Theme,
        size: [f32; 2],
        document: Document,
        ui_state: DocumentUiState,
        show_format_bar: bool,
        save_error: Option<DocumentError>,
    ) -> Self {
        let document = Rc::new(RefCell::new(document));
        let state = Rc::new(RefCell::new(GridState::default()));
        let ui_state = Rc::new(RefCell::new(ui_state));
        let storage = Rc::new(MemoryStorage::new(b"a,b\n1,2\n"));
        let locale = Locale::try_from_str("en").expect("English is a valid locale");

        let ui_document = Rc::clone(&document);
        let ui_state = Rc::clone(&ui_state);
        let ui_storage = Rc::clone(&storage);
        let ui_grid = Rc::clone(&state);
        let ui = UiHarness::new(theme, size, move |ui| {
            let mut document = ui_document.borrow_mut();
            let mut state = ui_grid.borrow_mut();
            let mut ui_state = ui_state.borrow_mut();
            let mut clipboard = TestClipboard;
            DocumentView {
                document: &mut document,
                state: &mut state,
                ui_state: &mut ui_state,
                storage: &*ui_storage,
                clipboard: &mut clipboard,
                language: Language::En,
                prefers_chinese: false,
                locale: &locale,
                decimal_separator: ".",
                show_format_bar,
                appearance: GridAppearance::default(),
                input_blocked: false,
                save_error,
            }
            .show(ui);
        });

        Self {
            ui,
            document,
            state,
            directory: None,
        }
    }

    /// Runs two frames so the status line and banners settle.
    pub fn settle(&mut self) {
        self.ui.run();
        self.ui.run();
    }

    /// Changes the document before the next frame.
    pub fn with_document_mut(&self, change: impl FnOnce(&mut Document)) {
        change(&mut self.document.borrow_mut());
    }

    /// Changes the grid selection or editor before the next frame.
    pub fn with_grid_mut(&self, change: impl FnOnce(&mut GridState)) {
        change(&mut self.state.borrow_mut());
    }

    /// Keeps a large fixture's file alive with the harness.
    pub fn keep(mut self, directory: tempfile::TempDir) -> Self {
        self.directory = Some(directory);
        self
    }

    /// Renders once and compares the frame with `name`.
    pub fn snapshot(&mut self, name: impl Into<String>, results: &mut SnapshotResults) {
        self.ui.snapshot(name);
        results.extend(self.ui.take_snapshot_results());
    }
}

/// Paints the harness background so a snapshot has no transparent area.
pub fn paint_background(ui: &mut egui::Ui, theme: tabtool_ui::Theme) {
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, theme.colors.window_background);
}

/// The format used by the header and column snapshots.
pub fn csv_configuration() -> CsvConfiguration {
    CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    }
}

/// A document with a header and three records.
pub fn csv_document() -> Document {
    let storage = MemoryStorage::new(CSV);
    loaded(&storage, csv_configuration())
}

//! Document, grid-selection and large-file snapshot states.

use std::path::Path;

use tabtool_core::{CsvConfiguration, Encoding};
use tabtool_document::{Document, DocumentError};
use tabtool_i18n::Language;
use tabtool_ui::document::{ChromeAlert, DocumentUiState, alert_message};
use tabtool_ui::grid::CellEditor;

use super::fixture::{DocumentFixture, csv_document};
use crate::support::{MemoryStorage, large_ready, loaded};

fn settings() -> DocumentUiState {
    DocumentUiState::default()
}

/// A new untitled 1×3 document.
pub fn new_document(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    DocumentFixture::new(
        theme,
        size,
        crate::support::untitled(CsvConfiguration::default()),
        settings(),
        true,
        None,
    )
}

/// A CSV grid with a header row and four records.
pub fn csv_with_header(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    DocumentFixture::new(theme, size, csv_document(), settings(), true, None)
}

/// The second record selected, as a row selection.
pub fn selected_row(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let fixture = csv_with_header(theme, size);
    fixture.with_grid_mut(|state| {
        state.selection.rows.insert(1);
    });
    fixture
}

/// The amount column selected.
pub fn selected_column(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let fixture = csv_with_header(theme, size);
    fixture.with_grid_mut(|state| {
        state.selection.columns.insert(1);
    });
    fixture
}

/// The amount cell in edit mode, with the caret at the end.
pub fn editing_cell(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let fixture = csv_with_header(theme, size);
    fixture.with_grid_mut(|state| {
        state.editing = Some(CellEditor::new(1, 1, "20"));
    });
    fixture
}

/// The format bar with non-default controls: a semicolon separator and a
/// decimal comma, so this state is distinct from the plain header grid.
pub fn format_bar(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let storage = MemoryStorage::new(b"name;amount;note\nApple;1,5;Fruit\nBanana;0,5;Fruit\n");
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        column_separator: ";".to_owned(),
        decimal_mark: ",".to_owned(),
        ..CsvConfiguration::default()
    };
    let document = loaded(&storage, configuration);
    DocumentFixture::new(theme, size, document, settings(), true, None)
}

/// The filter bar open with its default criterion.
pub fn filter_bar(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let fixture = csv_with_header(theme, size);
    fixture.with_document_mut(|document| {
        document.row_filter_mut().is_bar_visible = true;
    });
    fixture
}

/// A Windows-1252 document that cannot save a replacement character.
pub fn needs_attention(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let storage = MemoryStorage::new(b"a,b\n1,2\n");
    let mut document = loaded(
        &storage,
        CsvConfiguration {
            encoding: Encoding::Windows1252,
            ..CsvConfiguration::default()
        },
    );
    document.set_cell(0, 0, "\u{2192}");
    assert!(document.write_issue().is_some());
    DocumentFixture::new(theme, size, document, settings(), true, None)
}

/// The full-area read-failure overlay with Detect Automatically enabled.
pub fn read_error(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"\xFF",
            Some(Path::new("/virtual/document.csv")),
            Some(configuration),
            false,
        )
        .expect_err("the invalid bytes fail to decode");
    DocumentFixture::new(theme, size, document, settings(), true, None)
}

/// A failed save banner above the format bar.
pub fn save_error(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    DocumentFixture::new(
        theme,
        size,
        csv_document(),
        settings(),
        true,
        Some(DocumentError::CannotWriteFile),
    )
}

/// A one-shot format failure banner.
pub fn format_alert(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let ui_state = DocumentUiState {
        alert: Some(ChromeAlert {
            title: "Format Not Applied",
            message: alert_message(Language::En, DocumentError::CannotReadFile),
        }),
        ..settings()
    };
    DocumentFixture::new(theme, size, csv_document(), ui_state, true, None)
}

/// The discard confirmation before a format re-read.
pub fn reparse(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let ui_state = DocumentUiState {
        pending_reparse: Some(CsvConfiguration {
            column_separator: ";".to_owned(),
            ..CsvConfiguration::default()
        }),
        ..settings()
    };
    DocumentFixture::new(theme, size, csv_document(), ui_state, true, None)
}

fn large_file() -> (tempfile::TempDir, Document) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    std::fs::write(&path, b"a,b\n1,2\n3,4\n").expect("fixture file is written");
    let document = large_ready(&path);
    (directory, document)
}

/// A large document with pinned indexing progress.
pub fn large_indexing(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let (directory, mut document) = large_file();
    document.force_indexing_state_for_test(0.42);
    DocumentFixture::new(theme, size, document, settings(), true, None).keep(directory)
}

/// A large document with pinned streaming-save progress.
pub fn saving(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let (directory, mut document) = large_file();
    document.force_saving_state_for_test(0.42);
    DocumentFixture::new(theme, size, document, settings(), true, None).keep(directory)
}

/// A large document whose file grew after it was mapped.
pub fn stale(theme: tabtool_ui::Theme, size: [f32; 2]) -> DocumentFixture {
    let (directory, mut document) = large_file();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(directory.path().join("large.csv"))
        .expect("open the fixture for append");
    std::io::Write::write_all(&mut file, b"5,6\n").expect("append a row");
    document.refresh_large_staleness();
    DocumentFixture::new(theme, size, document, settings(), true, None).keep(directory)
}

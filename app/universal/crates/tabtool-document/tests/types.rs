//! Type-level port of `CSVDocumentTests.swift`: the defaults of a document
//! that never read a file, plus selection, sort, phase, error and notice
//! values. Loading behavior is covered by later test files.

use std::collections::BTreeSet;

use tabtool_core::{CsvConfiguration, CsvError, CsvWriteIssue, SortDirection};
use tabtool_document::{
    ActiveSort, Document, DocumentError, DocumentNotice, DocumentPhase, GridSelection, LargeStatus,
};

#[test]
fn a_new_document_starts_loading_with_a_blank_three_by_one_table() {
    let configuration = CsvConfiguration::default();
    let document = Document::new(configuration.clone());

    assert_eq!(document.phase(), &DocumentPhase::Loading);
    let table = document.table().expect("an untitled document has a table");
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.headers(), None);

    assert_eq!(document.configuration(), &configuration);
    assert_eq!(document.selection(), &GridSelection::default());
    assert!(document.selection().rows.is_empty());
    assert!(document.selection().columns.is_empty());
    assert!(document.active_sort().is_none());
    assert!(document.file_path().is_none());
    assert_eq!(document.revision(), 0);
    assert!(!document.is_dirty());
    assert!(document.write_issue().is_none());
    assert_eq!(document.large_status(), LargeStatus::None);
    assert!(!document.is_large_mode());
    assert!(document.has_content());
    assert!(document.can_edit_cells());
    assert_eq!(document.notice(), None);
}

#[test]
fn a_new_document_uses_the_injected_configuration() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        decimal_mark: ",".to_string(),
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };

    let document = Document::new(configuration.clone());

    assert_eq!(document.configuration(), &configuration);
    assert_eq!(document.phase(), &DocumentPhase::Loading);
    assert_eq!(document.table().expect("table").row_count(), 1);
}

#[test]
fn the_row_source_mirrors_the_table() {
    let document = Document::new(CsvConfiguration::default());
    let source = document.row_source().expect("an untitled document rows");

    assert_eq!(source.row_count(), 1);
    assert_eq!(source.column_count(), 3);
    assert_eq!(source.cell(0, 0), "");
}

#[test]
fn selection_starts_empty_and_is_mutated_in_place() {
    let mut document = Document::new(CsvConfiguration::default());
    assert!(document.selection().rows.is_empty());

    document.selection_mut().rows.insert(2);
    document.selection_mut().columns.insert(0);

    assert_eq!(document.selection().rows, BTreeSet::from([2]));
    assert_eq!(document.selection().columns, BTreeSet::from([0]));
}

#[test]
fn grid_selection_keeps_both_axes_independently() {
    let mut selection = GridSelection::default();
    selection.rows.insert(2);
    selection.rows.insert(0);
    selection.columns.insert(1);

    assert_eq!(selection.rows, BTreeSet::from([0, 2]));
    assert_eq!(selection.columns, BTreeSet::from([1]));
    assert_eq!(selection, selection.clone());
    assert_ne!(selection, GridSelection::default());
}

#[test]
fn active_sort_carries_column_direction_and_source_order() {
    let sort = ActiveSort {
        column_id: 7,
        direction: SortDirection::Descending,
        source_order: vec![2, 0, 1],
    };

    assert_eq!(sort.column_id, 7);
    assert_eq!(sort.direction, SortDirection::Descending);
    assert_eq!(sort.source_order, vec![2, 0, 1]);
    assert_eq!(sort, sort.clone());
}

#[test]
fn document_phase_carries_its_typed_cause() {
    assert_eq!(DocumentPhase::Loading, DocumentPhase::Loading);
    assert_ne!(DocumentPhase::Loading, DocumentPhase::Ready);
    assert_ne!(DocumentPhase::Ready, DocumentPhase::UnreadableFile);

    let read_failed = DocumentPhase::ReadFailed(CsvError::CannotDecodeData);
    assert_eq!(
        read_failed,
        DocumentPhase::ReadFailed(CsvError::CannotDecodeData)
    );
    assert_ne!(
        read_failed,
        DocumentPhase::ReadFailed(CsvError::MalformedQuotedField)
    );

    let needs_attention = DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput);
    assert_eq!(
        needs_attention,
        DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );
    assert_ne!(
        needs_attention,
        DocumentPhase::NeedsAttention(CsvWriteIssue::UnencodableText)
    );
}

#[test]
fn document_errors_are_distinct_values() {
    assert_eq!(DocumentError::NotReady, DocumentError::NotReady);
    assert_ne!(DocumentError::NotReady, DocumentError::RequiresConfirmation);
    assert_ne!(
        DocumentError::RequiresConfirmation,
        DocumentError::CannotReadFile
    );
    assert_ne!(DocumentError::CannotReadFile, DocumentError::NotReady);
}

#[test]
fn large_status_covers_every_swift_case() {
    assert_eq!(LargeStatus::None, LargeStatus::None);
    assert_eq!(LargeStatus::Indexing(0.5), LargeStatus::Indexing(0.5));
    assert_ne!(LargeStatus::Indexing(0.5), LargeStatus::Indexing(1.0));
    assert_eq!(LargeStatus::Ready, LargeStatus::Ready);
    assert_eq!(
        LargeStatus::Stale { appended: true },
        LargeStatus::Stale { appended: true }
    );
    assert_ne!(
        LargeStatus::Stale { appended: true },
        LargeStatus::Stale { appended: false }
    );
    assert_eq!(LargeStatus::Saving(0.25), LargeStatus::Saving(0.25));
    assert_ne!(LargeStatus::Saving(0.25), LargeStatus::Saving(0.0));
    assert_ne!(LargeStatus::Ready, LargeStatus::None);
}

#[test]
fn document_notice_carries_the_rejected_edit() {
    assert_eq!(
        DocumentNotice::EditRejected(CsvError::RowNotEditable),
        DocumentNotice::EditRejected(CsvError::RowNotEditable)
    );
    assert_ne!(
        DocumentNotice::EditRejected(CsvError::RowNotEditable),
        DocumentNotice::EditRejected(CsvError::UnencodableText)
    );
    assert_ne!(
        DocumentNotice::EditRejected(CsvError::RowNotEditable),
        DocumentNotice::CopyLimitExceeded
    );
    assert_eq!(
        DocumentNotice::CopyLimitExceeded,
        DocumentNotice::CopyLimitExceeded
    );
}

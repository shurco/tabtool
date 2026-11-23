//! Port of the snapshot and encoded-data cases in `CSVDocumentTests.swift`
//! and `CSVDocumentConcurrencyTests.swift`.

use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvError, CsvWriteIssue, CsvWriter};
use tabtool_document::{Document, DocumentError, DocumentPhase, StdStorage};

/// A non-default configuration used where a test supplies an explicit format.
fn supplied() -> CsvConfiguration {
    CsvConfiguration {
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    }
}

#[test]
fn snapshot_before_load_is_not_ready() {
    let document = Document::new(CsvConfiguration::default());

    assert_eq!(document.snapshot().unwrap_err(), DocumentError::NotReady);
    assert_eq!(document.encoded_data(), Err(DocumentError::NotReady));
}

#[test]
fn snapshot_captures_the_table_configuration_and_bom() {
    let configuration = supplied();
    let mut input = vec![0xEF, 0xBB, 0xBF];
    input.extend_from_slice(b"a,b\n1,2\n");
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(&input, None, Some(configuration.clone()), false)
        .expect("load byte-order-marked data");

    let snapshot = document.snapshot().expect("snapshot");

    assert!(Arc::ptr_eq(
        &snapshot.table,
        document.table().expect("table")
    ));
    assert_eq!(snapshot.configuration, configuration);
    assert!(snapshot.includes_utf8_bom);
    assert_eq!(snapshot.table.cell(1, 1), "2");
}

#[test]
fn encoded_data_round_trips_through_the_snapshot() {
    let mut document = Document::new(supplied());
    document
        .load_from_bytes(
            b"a,b,c\n1,2,3\n\"1,1\",\"1,2\",\"1,3\"\n",
            None,
            Some(supplied()),
            false,
        )
        .expect("load data");
    let snapshot = document.snapshot().expect("snapshot");

    let mut writer = CsvWriter::new(&snapshot.table, None, &snapshot.configuration);
    writer.line_ending = snapshot.line_ending;
    let expected = writer.data().expect("write snapshot");

    assert_eq!(document.encoded_data(), Ok(expected));
    assert_eq!(
        document.encoded_data(),
        Ok(b"\"a\",\"b\",\"c\"\n\"1\",\"2\",\"3\"\n\"1,1\",\"1,2\",\"1,3\"\n".to_vec())
    );
}

#[test]
fn the_utf8_bom_is_written_back_from_the_snapshot() {
    let mut input = vec![0xEF, 0xBB, 0xBF];
    input.extend_from_slice(b"a,b");
    let mut document = Document::new(supplied());

    document
        .load_from_bytes(&input, None, Some(supplied()), false)
        .expect("load byte-order-marked data");

    assert!(document.snapshot().expect("snapshot").includes_utf8_bom);
    assert!(
        document
            .encoded_data()
            .expect("encoded data")
            .starts_with(&[0xEF, 0xBB, 0xBF])
    );
}

#[test]
fn encoded_data_writes_the_header_row_when_configured() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..supplied()
    };
    let mut document = Document::new(configuration.clone());

    document
        .load_from_bytes(b"a,b\n1,2\n", None, Some(configuration), false)
        .expect("load data");

    let snapshot = document.snapshot().expect("snapshot");
    assert_eq!(
        snapshot.table.headers(),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(
        document.encoded_data(),
        Ok(b"\"a\",\"b\"\n\"1\",\"2\"\n".to_vec())
    );
}

#[test]
fn encoded_data_reports_the_write_issue_for_ambiguous_output() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"a\n\n", None, Some(configuration), false)
        .expect("load ambiguous content");

    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );
    assert!(document.snapshot().is_ok());
    assert_eq!(
        document.encoded_data(),
        Err(DocumentError::Csv(CsvError::AmbiguousOutput))
    );
}

#[test]
fn encoded_data_without_a_table_fails() {
    let mut document = Document::new(supplied());
    assert!(
        document
            .load_from_bytes(&[0xFF, 0xFD], None, Some(supplied()), false)
            .is_err()
    );
    assert!(document.table().is_none());

    assert_eq!(document.snapshot().unwrap_err(), DocumentError::NotReady);
    assert_eq!(document.encoded_data(), Err(DocumentError::NotReady));
}

#[test]
fn snapshot_after_an_unreadable_file_is_not_ready() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("missing.csv");
    let mut document = Document::new(supplied());

    assert_eq!(
        document.load_from_path(&path, &StdStorage, None, false),
        Err(DocumentError::CannotReadFile)
    );
    assert_eq!(document.phase(), &DocumentPhase::UnreadableFile);

    assert_eq!(document.snapshot().unwrap_err(), DocumentError::NotReady);
    assert_eq!(document.encoded_data(), Err(DocumentError::NotReady));
}

#[test]
fn a_new_document_snapshots_its_blank_table_and_empty_headers() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..supplied()
    };
    let mut document = Document::new(CsvConfiguration::default());
    document
        .new_document(configuration.clone())
        .expect("replace with a new document");

    let snapshot = document.snapshot().expect("snapshot");

    assert!(Arc::ptr_eq(
        &snapshot.table,
        document.table().expect("table")
    ));
    assert_eq!(snapshot.configuration, configuration);
    assert!(!snapshot.includes_utf8_bom);
    assert_eq!(
        snapshot.table.headers(),
        Some(vec![String::new(), String::new(), String::new()])
    );
    assert_eq!(
        document.encoded_data(),
        Ok(b"\"\",\"\",\"\"\n\"\",\"\",\"\"".to_vec())
    );
}

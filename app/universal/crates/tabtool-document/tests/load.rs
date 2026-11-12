//! Port of the load cases in `CSVDocumentTests.swift` and the load invariants
//! in `CSVDocumentConcurrencyTests.swift`.
//!
//! `None` means "detect the format from the bytes", matching Swift's
//! `configuration: nil` initializer. `Some`, including
//! `Some(CsvConfiguration::default())`, is used verbatim.

use std::path::Path;

use tabtool_core::{CsvConfiguration, CsvError, CsvWriteIssue, Encoding};
use tabtool_document::{Document, DocumentError, DocumentPhase, StdStorage};

/// A non-default configuration used where a test supplies an explicit format.
fn supplied() -> CsvConfiguration {
    CsvConfiguration {
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    }
}

#[test]
fn an_empty_file_loads_as_a_one_by_one_table() {
    let mut document = Document::new(CsvConfiguration::default());

    assert_eq!(document.load_from_bytes(b"", None, None, false), Ok(()));

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    let table = document.table().expect("the empty file has a table");
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.cell(0, 0), "");
    assert_eq!(table.headers(), None);
    assert!(document.file_path().is_none());
    // Untitled content stays dirty until Save As binds a file.
    assert!(document.is_dirty());
}

#[test]
fn load_uses_a_supplied_configuration() {
    let configuration = supplied();
    let mut document = Document::new(configuration.clone());
    let data = b"a,b,c\n1,2,3\n\"1,1\",\"1,2\",\"1,3\"\n";

    assert_eq!(
        document.load_from_bytes(data, None, Some(configuration.clone()), false),
        Ok(())
    );

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.configuration(), &configuration);
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 3);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.headers(), None);
    assert_eq!(table.cell(2, 0), "1,1");
}

#[test]
fn load_detects_the_format_when_the_configuration_is_default() {
    let mut document = Document::new(CsvConfiguration::default());

    assert_eq!(
        document.load_from_bytes(b"a;b\n1,5;2,5\n", None, None, false),
        Ok(())
    );

    assert_eq!(document.configuration().column_separator, ";");
    assert_eq!(document.configuration().decimal_mark, ",");
    assert!(document.configuration().first_row_as_header);
    let table = document.table().expect("table");
    assert_eq!(
        table.headers(),
        Some(vec!["a".to_string(), "b".to_string()])
    );
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.cell(0, 1), "2,5");
}

#[test]
fn an_explicit_default_configuration_skips_detection() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("semicolons.csv");
    // Detection would pick the semicolon separator and comma decimal mark.
    std::fs::write(&path, "a;b\n1,5;2,5\n").expect("write sample file");
    let mut document = Document::new(CsvConfiguration::default());

    assert_eq!(
        document.load_from_path(&path, &StdStorage, Some(CsvConfiguration::default()), false),
        Ok(())
    );

    assert_eq!(document.configuration(), &CsvConfiguration::default());
    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.cell(0, 0), "a;b");
    assert_eq!(table.cell(1, 0), "1");
}

#[test]
fn detection_prefers_gbk_when_chinese_encoding_is_preferred() {
    // "日本" in GBK; the same bytes also decode as CP1252 text.
    let bytes = b"a\n\xC8\xD5\xB1\xBE\n";

    let mut chinese = Document::new(CsvConfiguration::default());
    assert_eq!(chinese.load_from_bytes(bytes, None, None, true), Ok(()));
    assert_eq!(chinese.configuration().encoding, Encoding::Gbk);
    assert_eq!(
        chinese.table().expect("table").cell(0, 0),
        "日本".to_string()
    );

    let mut western = Document::new(CsvConfiguration::default());
    assert_eq!(western.load_from_bytes(bytes, None, None, false), Ok(()));
    assert_eq!(western.configuration().encoding, Encoding::Windows1252);
}

#[test]
fn a_supplied_no_quote_configuration_reports_ambiguous_output() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());

    assert_eq!(
        document.load_from_bytes(b"a\n\n", None, Some(configuration.clone()), false),
        Ok(())
    );

    // Detection would pick a quoted configuration and report `Ready`; using
    // the supplied configuration is what makes the loaded content ambiguous.
    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );
    assert_eq!(document.write_issue(), Some(CsvWriteIssue::AmbiguousOutput));
    assert!(document.table().is_some());
}

#[test]
fn invalid_bytes_fail_the_read_and_leave_no_table() {
    let configuration = supplied();
    let mut document = Document::new(configuration.clone());

    assert_eq!(
        document.load_from_bytes(&[0xFF, 0xFD], None, Some(configuration), false),
        Err(DocumentError::NotReady)
    );

    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::CannotDecodeData)
    );
    assert!(document.table().is_none());
}

#[test]
fn load_from_path_reads_the_bytes_and_records_the_path() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("data.csv");
    std::fs::write(&path, "a;b\n1;2\n").expect("write sample file");
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());

    assert_eq!(
        document.load_from_path(&path, &StdStorage, Some(configuration), false),
        Ok(())
    );

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.file_path(), Some(path.as_path()));
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.cell(1, 1), "2");
}

#[test]
fn a_missing_file_is_unreadable_not_a_parse_failure() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("missing.csv");
    let mut document = Document::new(supplied());

    assert_eq!(
        document.load_from_path(&path, &StdStorage, None, false),
        Err(DocumentError::CannotReadFile)
    );

    assert_eq!(document.phase(), &DocumentPhase::UnreadableFile);
    assert!(document.table().is_none());
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn load_from_path_reports_a_parse_error_without_a_table() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("invalid.csv");
    std::fs::write(&path, [0xFF, 0xFD]).expect("write invalid bytes");
    let mut document = Document::new(supplied());

    assert_eq!(
        document.load_from_path(&path, &StdStorage, Some(supplied()), false),
        Err(DocumentError::NotReady)
    );

    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::CannotDecodeData)
    );
    assert!(document.table().is_none());
    assert_eq!(document.file_path(), Some(path.as_path()));
}

#[test]
fn a_failed_load_can_be_retried_from_its_file() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("retry.csv");
    std::fs::write(&path, [0xFF, 0xFD]).expect("write invalid bytes");
    let mut document = Document::new(supplied());

    assert_eq!(
        document.load_from_bytes(&[0xFF, 0xFD], Some(&path), Some(supplied()), false),
        Err(DocumentError::NotReady)
    );
    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::CannotDecodeData)
    );

    std::fs::write(&path, "a,b\nc,d\n").expect("write valid bytes");
    assert_eq!(
        document.load_from_path(&path, &StdStorage, Some(supplied()), false),
        Ok(())
    );

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.cell(0, 0), "a");
}

#[test]
fn a_second_load_does_not_replace_published_content() {
    let mut document = Document::new(supplied());
    assert_eq!(
        document.load_from_bytes(b"a,b\n", None, Some(supplied()), false),
        Ok(())
    );
    let first = document.table().expect("table").clone();
    let revision = document.revision();
    let phase = *document.phase();

    assert_eq!(
        document.load_from_bytes(b"x,y,z\n", Some(Path::new("/tmp/other.csv")), None, false),
        Ok(())
    );

    assert_eq!(document.table(), Some(&first));
    assert_eq!(document.revision(), revision);
    assert_eq!(document.phase(), &phase);
    assert!(document.file_path().is_none());
}

#[test]
fn a_failed_load_does_not_publish_a_partial_table() {
    let mut document = Document::new(supplied());

    assert!(
        document
            .load_from_bytes(b"a,b\n\"unterminated\n", None, Some(supplied()), false)
            .is_err()
    );

    assert_eq!(
        document.phase(),
        &DocumentPhase::ReadFailed(CsvError::MalformedQuotedField)
    );
    assert!(document.table().is_none());
}

#[test]
fn new_document_is_three_columns_by_one_row_and_ready() {
    let mut document = Document::new(CsvConfiguration::default());

    document.new_document(CsvConfiguration::default());

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.headers(), None);
    assert!(document.file_path().is_none());
    assert!(document.is_dirty());
}

#[test]
fn new_document_with_a_header_preference_gets_three_empty_headers() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(CsvConfiguration::default());

    document.new_document(configuration.clone());

    assert_eq!(document.configuration(), &configuration);
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    let table = document.table().expect("table");
    assert_eq!(
        table.headers(),
        Some(vec![String::new(), String::new(), String::new()])
    );
    assert_eq!(table.row_count(), 1);
}

#[test]
fn new_document_uses_the_injected_configuration() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(CsvConfiguration::default());

    document.new_document(configuration.clone());

    assert_eq!(document.configuration(), &configuration);
    assert_eq!(document.phase(), &DocumentPhase::Ready);
}

#[test]
fn setting_a_path_binds_the_document_and_none_makes_it_untitled() {
    let path = Path::new("/tmp/tabtool-load-test.csv");
    let mut document = Document::new(supplied());
    assert_eq!(
        document.load_from_bytes(b"a,b\n", Some(path), Some(supplied()), false),
        Ok(())
    );
    let table = document.table().expect("table").clone();
    assert_eq!(document.file_path(), Some(path));

    document.set_file_path(None);

    assert!(document.file_path().is_none());
    assert_eq!(document.table(), Some(&table));
}

#[test]
fn loading_decodes_utf16_bytes_with_a_supplied_encoding() {
    let configuration = CsvConfiguration {
        encoding: Encoding::Utf16Le,
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    // "a,b" as little-endian UTF-16 code units.
    let bytes = [0x61, 0x00, 0x2C, 0x00, 0x62, 0x00];
    let mut document = Document::new(configuration.clone());

    assert_eq!(
        document.load_from_bytes(&bytes, None, Some(configuration), false),
        Ok(())
    );

    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.table().expect("table").cell(0, 1), "b".to_string());
}

//! Ports of the Apple `CSVExportTests` suite, plus the Review Focus cases for
//! copy-only conversion, generated headers and preflight after conversion.

use tabtool_core::{
    CsvConfiguration, CsvError, CsvTable, CsvWriteIssue, Encoding, ExportInput, export_data,
    export_preflight_issue,
};

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect()
}

fn headers(values: &[&str]) -> Option<Vec<String>> {
    Some(values.iter().map(|name| (*name).to_owned()).collect())
}

/// Mirrors Swift's `CSVTable(rows:headers:)` default minimum of one column.
fn table(rows: Vec<Vec<String>>, headers: Option<Vec<String>>) -> CsvTable {
    CsvTable::from_rows(rows, headers, 1)
}

fn document(decimal_mark: &str) -> CsvConfiguration {
    CsvConfiguration {
        decimal_mark: decimal_mark.to_owned(),
        ..CsvConfiguration::default()
    }
}

fn no_quote() -> CsvConfiguration {
    CsvConfiguration {
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    }
}

fn export_with(
    table: &CsvTable,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
    allow_ambiguous_output: bool,
) -> Result<Vec<u8>, CsvError> {
    export_data(
        &ExportInput {
            table,
            document_configuration,
            export_configuration,
        },
        allow_ambiguous_output,
    )
}

fn export(
    table: &CsvTable,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
) -> String {
    let data = export_with(table, document_configuration, export_configuration, false)
        .expect("export succeeds");
    String::from_utf8(data).expect("export is UTF-8")
}

fn preflight(
    table: &CsvTable,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
) -> Option<CsvWriteIssue> {
    export_preflight_issue(&ExportInput {
        table,
        document_configuration,
        export_configuration,
    })
}

#[test]
fn export_converts_decimal_mark_and_preserves_formatting() {
    let t = table(rows(&[&["001,50"]]), None);
    let doc = CsvConfiguration {
        column_separator: ";".to_owned(),
        decimal_mark: ",".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(export(&t, &doc, &document(".")), "\"001.50\"");
}

#[test]
fn export_header_falls_back_to_generated_names() {
    let t = table(rows(&[&["x", "y"]]), None);
    let out = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export(&t, &CsvConfiguration::default(), &out),
        "\"A\",\"B\"\n\"x\",\"y\""
    );
}

#[test]
fn export_header_toggle_off_omits_header() {
    let t = table(rows(&[&["x", "y"]]), headers(&["h1", "h2"]));
    let out = CsvConfiguration {
        first_row_as_header: false,
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export(&t, &CsvConfiguration::default(), &out),
        "\"x\",\"y\""
    );
}

#[test]
fn export_uses_document_headers() {
    let t = table(rows(&[&["x", "y"]]), headers(&["h1", "h2"]));
    let out = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export(&t, &CsvConfiguration::default(), &out),
        "\"h1\",\"h2\"\n\"x\",\"y\""
    );
}

#[test]
fn export_converts_a_copy_only() {
    let t = table(rows(&[&["1,5", "text,5"]]), None);
    let data = export(&t, &document(","), &document("."));
    assert_eq!(data, "\"1.5\",\"text,5\"");
    assert_eq!(t.cell(0, 0), "1,5");
    assert_eq!(t.cell(0, 1), "text,5");
}

#[test]
fn export_propagates_ambiguity_flag() {
    let t = table(rows(&[&["a,b"]]), None);
    let no_quote_configuration = no_quote();
    assert_eq!(
        export_with(
            &t,
            &CsvConfiguration::default(),
            &no_quote_configuration,
            false
        ),
        Err(CsvError::AmbiguousOutput)
    );

    let data = export_with(
        &t,
        &CsvConfiguration::default(),
        &no_quote_configuration,
        true,
    )
    .unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "a,b");
}

#[test]
fn export_rejects_trailing_line_feed() {
    let t = table(rows(&[&["a"], &[""]]), None);
    let no_quote_configuration = no_quote();
    assert_eq!(
        export_with(
            &t,
            &CsvConfiguration::default(),
            &no_quote_configuration,
            false
        ),
        Err(CsvError::AmbiguousOutput)
    );

    let data = export_with(
        &t,
        &CsvConfiguration::default(),
        &no_quote_configuration,
        true,
    )
    .unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "a\n");
}

#[test]
fn export_uses_export_configuration_for_quoting() {
    let t = table(rows(&[&["1.5"]]), None);
    let doc = CsvConfiguration::default();
    let comma_out = CsvConfiguration {
        quote_character: String::new(),
        decimal_mark: ",".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export_with(&t, &doc, &comma_out, false),
        Err(CsvError::AmbiguousOutput)
    );

    let semicolon_out = CsvConfiguration {
        column_separator: ";".to_owned(),
        quote_character: String::new(),
        decimal_mark: ",".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(export(&t, &doc, &semicolon_out), "1,5");
}

#[test]
fn export_preflight_uses_converted_decimal_mark() {
    let t = table(rows(&[&["1.5"]]), None);
    let export_configuration = CsvConfiguration {
        decimal_mark: ",".to_owned(),
        ..no_quote()
    };
    assert_eq!(
        preflight(&t, &document("."), &export_configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
}

#[test]
fn export_preflight_ignores_ambiguity_removed_by_conversion() {
    let t = table(rows(&[&["1,5"]]), None);
    let document_configuration = document(",");
    let export_configuration = CsvConfiguration {
        decimal_mark: ".".to_owned(),
        ..no_quote()
    };
    assert_eq!(
        preflight(&t, &document_configuration, &export_configuration),
        None
    );
    assert_eq!(
        export(&t, &document_configuration, &export_configuration),
        "1.5"
    );
}

#[test]
fn export_preflight_allows_forced_ambiguous_converted_output() {
    let t = table(rows(&[&["1.5"]]), None);
    let document_configuration = document(".");
    let export_configuration = CsvConfiguration {
        decimal_mark: ",".to_owned(),
        ..no_quote()
    };
    assert_eq!(
        preflight(&t, &document_configuration, &export_configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );

    let data = export_with(&t, &document_configuration, &export_configuration, true).unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "1,5");
}

#[test]
fn export_preflight_reports_unencodable_text() {
    let t = table(rows(&[&["ж"]]), None);
    let export_configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..CsvConfiguration::default()
    };
    assert_eq!(
        preflight(&t, &CsvConfiguration::default(), &export_configuration),
        Some(CsvWriteIssue::UnencodableText)
    );
}

#[test]
fn export_with_empty_export_mark_strips_the_mark() {
    let t = table(rows(&[&["1.5", "x"]]), None);
    let document_configuration = document(".");
    let export_configuration = document("");
    assert_eq!(
        preflight(&t, &document_configuration, &export_configuration),
        None
    );
    assert_eq!(
        export(&t, &document_configuration, &export_configuration),
        "\"15\",\"x\""
    );
    assert_eq!(t.cell(0, 0), "1.5");
}

#[test]
fn export_preflight_checks_the_written_header() {
    let t = table(rows(&[&["x"]]), headers(&["a,b"]));
    let export_configuration = CsvConfiguration {
        first_row_as_header: true,
        ..no_quote()
    };
    assert_eq!(
        preflight(&t, &CsvConfiguration::default(), &export_configuration),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
}

#[test]
fn export_preflight_ignores_an_unwritten_header() {
    let t = table(rows(&[&["x"]]), headers(&["a,b"]));
    assert_eq!(
        preflight(&t, &CsvConfiguration::default(), &no_quote()),
        None
    );
}

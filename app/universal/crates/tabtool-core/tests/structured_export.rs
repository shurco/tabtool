//! Ports of the Apple `StructuredExportTests` suite: JSON and XML export
//! through `export_data`.

use tabtool_core::{
    CsvConfiguration, CsvTable, Encoding, ExportFormat, ExportInput, export_data,
    export_preflight_issue,
};

fn table(rows: &[&[&str]], headers: Option<&[&str]>) -> CsvTable {
    CsvTable::from_rows(
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
        headers.map(|names| names.iter().map(|name| (*name).to_owned()).collect()),
        1,
    )
}

fn export(table: &CsvTable, format: ExportFormat) -> String {
    export_with(
        table,
        format,
        &CsvConfiguration::default(),
        &CsvConfiguration::default(),
    )
}

fn export_with(
    table: &CsvTable,
    format: ExportFormat,
    document_configuration: &CsvConfiguration,
    export_configuration: &CsvConfiguration,
) -> String {
    let data = export_data(
        &ExportInput {
            table,
            format,
            document_configuration,
            export_configuration,
        },
        false,
    )
    .expect("structured export succeeds");
    String::from_utf8(data).expect("structured export is UTF-8")
}

#[test]
fn json_writes_an_array_of_objects_in_column_order() {
    let t = table(&[&["Ann", "1.5"], &["Bob", "2"]], Some(&["Name", "Price"]));
    assert_eq!(
        export(&t, ExportFormat::Json),
        "[\n  {\"Name\": \"Ann\", \"Price\": \"1.5\"},\n  {\"Name\": \"Bob\", \"Price\": \"2\"}\n]\n"
    );
}

#[test]
fn json_of_an_empty_table_is_an_empty_array() {
    let t = table(&[], Some(&["A"]));
    assert_eq!(export(&t, ExportFormat::Json), "[]\n");
}

#[test]
fn json_keys_fall_back_to_column_letters_and_stay_unique() {
    let t = table(&[&["1", "2", "3"]], Some(&["k", "", "k"]));
    assert_eq!(
        export(&t, ExportFormat::Json),
        "[\n  {\"k\": \"1\", \"B\": \"2\", \"k_2\": \"3\"}\n]\n"
    );
    let t = table(&[&["1", "2"]], None);
    assert!(export(&t, ExportFormat::Json).contains("{\"A\": \"1\", \"B\": \"2\"}"));
}

#[test]
fn json_round_trips_through_a_parser() {
    let t = table(&[&["a\"\n\\\u{1}", "007"]], Some(&["k", "k"]));
    let parsed: serde_json::Value =
        serde_json::from_str(&export(&t, ExportFormat::Json)).expect("valid JSON");
    assert_eq!(
        parsed,
        serde_json::json!([{ "k": "a\"\n\\\u{1}", "k_2": "007" }])
    );
}

#[test]
fn xml_writes_row_elements_named_by_headers() {
    let t = table(&[&["Ann", "1.5"]], Some(&["Name", "Price"]));
    assert_eq!(
        export(&t, ExportFormat::Xml),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rows>\n  <row>\n    <Name>Ann</Name>\n    <Price>1.5</Price>\n  </row>\n</rows>\n"
    );
}

#[test]
fn xml_of_an_empty_table_has_only_the_root() {
    let t = table(&[], Some(&["A"]));
    assert_eq!(
        export(&t, ExportFormat::Xml),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rows>\n</rows>\n"
    );
}

#[test]
fn xml_names_stay_unique_after_sanitizing() {
    let t = table(&[&["1", "2"]], Some(&["a b", "a_b"]));
    let xml = export(&t, ExportFormat::Xml);
    assert!(xml.contains("<a_b>1</a_b>"), "{xml}");
    assert!(xml.contains("<a_b_2>2</a_b_2>"), "{xml}");
}

#[test]
fn structured_formats_convert_the_decimal_mark() {
    let t = table(&[&["1,5"]], Some(&["Price"]));
    let document = CsvConfiguration {
        column_separator: ";".to_owned(),
        decimal_mark: ",".to_owned(),
        ..CsvConfiguration::default()
    };
    let export_configuration = CsvConfiguration::default();
    assert!(
        export_with(&t, ExportFormat::Json, &document, &export_configuration)
            .contains("\"Price\": \"1.5\"")
    );
    assert!(
        export_with(&t, ExportFormat::Xml, &document, &export_configuration)
            .contains("<Price>1.5</Price>")
    );
}

/// JSON and XML are always UTF-8, so the CSV encoding and quoting rules never
/// block them.
#[test]
fn structured_formats_ignore_csv_write_rules() {
    let t = table(&[&["ж,\n"]], None);
    let export_configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    for format in [ExportFormat::Json, ExportFormat::Xml] {
        let input = ExportInput {
            table: &t,
            format,
            document_configuration: &CsvConfiguration::default(),
            export_configuration: &export_configuration,
        };
        assert_eq!(export_preflight_issue(&input), None, "{format:?}");
        let text =
            String::from_utf8(export_data(&input, false).expect("export succeeds")).expect("UTF-8");
        assert!(text.contains("ж,"), "{format:?}: {text}");
    }
}

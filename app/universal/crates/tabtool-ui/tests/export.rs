//! Ports of the Apple `ExportPlanTests` decisions and the export panel.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use tabtool_core::{CsvConfiguration, CsvParser, CsvTable, CsvWriteIssue, Encoding, ExportFormat};
use tabtool_ui::export::{self, ExportPanelAction, export_panel};
use tabtool_ui::harness::kittest::{NodeT, Queryable};
use tabtool_ui::{Theme, UiHarness};

fn table(rows: &[&[&str]], headers: Option<&[&str]>) -> CsvTable {
    CsvTable::from_rows(
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
        headers.map(|names| names.iter().map(|name| (*name).to_owned()).collect()),
        1,
    )
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

#[test]
fn base_name_appends_the_export_suffix() {
    assert_eq!(
        export::base_name(Some(Path::new("/tmp/Sales.csv"))),
        "Sales-export"
    );
    assert_eq!(
        export::base_name(Some(Path::new("/tmp/Quarter.Sales.csv"))),
        "Quarter.Sales-export"
    );
}

#[test]
fn base_name_falls_back_to_untitled() {
    assert_eq!(export::base_name(None), "Untitled-export");
    assert_eq!(export::base_name(Some(Path::new("/"))), "Untitled-export");
}

#[test]
fn the_filename_uses_csv_for_non_tab_separators() {
    for separator in [",", ";", "|"] {
        let configuration = CsvConfiguration {
            column_separator: separator.to_owned(),
            ..CsvConfiguration::default()
        };
        assert_eq!(
            export::filename("Sales-export", ExportFormat::Csv, &configuration),
            "Sales-export.csv"
        );
        assert_eq!(
            export::file_extension(ExportFormat::Csv, &configuration),
            "csv"
        );
    }
}

#[test]
fn the_filename_uses_tsv_for_a_tab_separator() {
    let configuration = CsvConfiguration {
        column_separator: "\t".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export::filename("Sales-export", ExportFormat::Csv, &configuration),
        "Sales-export.tsv"
    );
    assert_eq!(
        export::file_extension(ExportFormat::Csv, &configuration),
        "tsv"
    );
}

#[test]
fn converted_decimal_ambiguity_asks_for_confirmation() {
    let table = table(&[&["1.5"]], None);
    let document = document(".");
    let export = CsvConfiguration {
        decimal_mark: ",".to_owned(),
        ..no_quote()
    };

    assert_eq!(
        export::preflight(&table, ExportFormat::Csv, &document, &export),
        Some(CsvWriteIssue::AmbiguousOutput)
    );
}

#[test]
fn converted_decimal_output_is_not_blocked_by_original_ambiguity() {
    let table = table(&[&["1,5"]], None);
    let document = document(",");
    let export = CsvConfiguration {
        decimal_mark: ".".to_owned(),
        ..no_quote()
    };

    assert_eq!(
        export::preflight(&table, ExportFormat::Csv, &document, &export),
        None
    );
}

#[test]
fn export_data_converts_a_copy_only() {
    let table = table(&[&["1,5", "text,5"]], None);
    let data = export::data(
        &table,
        ExportFormat::Csv,
        &document(","),
        &document("."),
        false,
    )
    .expect("the export succeeds");
    assert_eq!(
        String::from_utf8(data).expect("UTF-8"),
        "\"1.5\",\"text,5\""
    );
    assert_eq!(table.cell(0, 0), "1,5");
    assert_eq!(table.cell(0, 1), "text,5");
}

#[test]
fn writing_and_reimporting_preserves_values_and_header() {
    let table = table(
        &[&["Alice", "1.50", "hello"], &["Bob", "2.00", "world"]],
        Some(&["Name", "Amount", "Note"]),
    );
    let export = CsvConfiguration {
        column_separator: ";".to_owned(),
        decimal_mark: ",".to_owned(),
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let document = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };

    let data = export::data(&table, ExportFormat::Csv, &document, &export, false)
        .expect("the export succeeds");
    let text = String::from_utf8(data.clone()).expect("UTF-8");
    assert_eq!(
        text,
        "\"Name\";\"Amount\";\"Note\"\n\"Alice\";\"1,50\";\"hello\"\n\"Bob\";\"2,00\";\"world\""
    );

    let mut parser = CsvParser::from_bytes(&data, &export).expect("the export parses");
    let rows = parser.read_all().expect("the export is complete");
    assert_eq!(rows[0], ["Name", "Amount", "Note"]);
    assert_eq!(rows[1], ["Alice", "1,50", "hello"]);
    assert_eq!(rows[2], ["Bob", "2,00", "world"]);
    assert_eq!(table.cell(0, 1), "1.50", "the source table is untouched");
}

#[test]
fn options_default_to_the_document_separator_and_csv() {
    let configuration = CsvConfiguration::default();
    assert_eq!(
        export::filename("Untitled-export", ExportFormat::Csv, &configuration),
        "Untitled-export.csv"
    );
    assert_eq!(
        export::file_extension(ExportFormat::Csv, &configuration),
        "csv"
    );

    let tab = CsvConfiguration {
        column_separator: "\t".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export::filename("Sales-export", ExportFormat::Csv, &tab),
        "Sales-export.tsv"
    );
    assert_eq!(export::file_extension(ExportFormat::Csv, &tab), "tsv");
}

fn panel_harness(
    options: &Rc<RefCell<CsvConfiguration>>,
    actions: &Rc<RefCell<Vec<ExportPanelAction>>>,
) -> UiHarness<'static> {
    panel_harness_with_format(&Rc::new(RefCell::new(ExportFormat::Csv)), options, actions)
}

fn panel_harness_with_format(
    format: &Rc<RefCell<ExportFormat>>,
    options: &Rc<RefCell<CsvConfiguration>>,
    actions: &Rc<RefCell<Vec<ExportPanelAction>>>,
) -> UiHarness<'static> {
    let format = Rc::clone(format);
    let options = Rc::clone(options);
    let actions = Rc::clone(actions);
    UiHarness::compact(Theme::desktop_light(), move |ui| {
        let mut format = format.borrow_mut();
        let mut options = options.borrow_mut();
        if let Some(action) = export_panel(
            ui,
            "export-panel",
            &mut format,
            &mut options,
            tabtool_i18n::Language::En,
        ) {
            actions.borrow_mut().push(action);
        }
    })
}

#[test]
fn the_panel_shows_every_export_control_and_the_header_note() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    for label in [
        "Format",
        "Encoding",
        "Separator",
        "Quote",
        "Decimal",
        "Header",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "missing control {label}"
        );
    }
    assert!(harness.query_by_label("Write Header Row").is_some());
    assert!(
        harness
            .query_by_label("The header is written only to the exported file.")
            .is_some()
    );
    assert_eq!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Separator")
            .value()
            .as_deref(),
        Some("Comma (,)")
    );
}

#[test]
fn changing_the_separator_updates_the_candidate_and_its_extension() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Separator")
        .click();
    harness.run();
    harness.get_by_label("Tab (⇥)").click();
    harness.run();

    assert_eq!(options.borrow().column_separator, "\t");
    assert_eq!(
        export::file_extension(ExportFormat::Csv, &options.borrow()),
        "tsv"
    );
}

#[test]
fn changing_the_quote_style_updates_both_quote_and_escape() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Quote")
        .click();
    harness.run();
    harness.get_by_label("None").click();
    harness.run();

    assert!(options.borrow().quote_character.is_empty());
    assert!(options.borrow().escape_character.is_empty());
}

#[test]
fn the_header_toggle_changes_only_the_export_candidate() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "Write Header Row")
        .click();
    harness.run();

    assert!(options.borrow().first_row_as_header);
}

#[test]
fn the_export_button_reports_the_candidate_and_cancel_reports_cancel() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.run();

    match actions.borrow().as_slice() {
        [ExportPanelAction::Export(format, configuration)] => {
            assert_eq!(*format, ExportFormat::Csv);
            assert_eq!(configuration, &CsvConfiguration::default());
        }
        other => panic!("expected one export action, got {other:?}"),
    }

    actions.borrow_mut().clear();
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Cancel")
        .click();
    harness.run();
    assert_eq!(actions.borrow().as_slice(), [ExportPanelAction::Cancel]);
}

#[test]
fn the_panel_encoding_list_is_the_supported_set() {
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness(&options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Encoding")
        .click();
    harness.run();
    harness.get_by_label("Western (Windows Latin 1)").click();
    harness.run();

    assert_eq!(options.borrow().encoding, Encoding::Windows1252);
}

#[test]
fn structured_formats_use_their_own_extension() {
    let tab = CsvConfiguration {
        column_separator: "\t".to_owned(),
        ..CsvConfiguration::default()
    };
    assert_eq!(
        export::filename("Sales-export", ExportFormat::Json, &tab),
        "Sales-export.json"
    );
    assert_eq!(
        export::filename("Sales-export", ExportFormat::Xml, &tab),
        "Sales-export.xml"
    );
    assert_eq!(export::file_extension(ExportFormat::Json, &tab), "json");
    assert_eq!(export::file_extension(ExportFormat::Xml, &tab), "xml");
}

/// Settings that make CSV ambiguous or unencodable do not apply to JSON and
/// XML, which are always UTF-8 with every value escaped.
#[test]
fn structured_formats_proceed_and_write_utf8() {
    let table = table(&[&["ж;"]], Some(&["Name"]));
    let export = CsvConfiguration {
        column_separator: ";".to_owned(),
        encoding: Encoding::Windows1252,
        ..no_quote()
    };
    assert_eq!(
        export::preflight(&table, ExportFormat::Json, &document("."), &export),
        None
    );
    let data = export::data(&table, ExportFormat::Json, &document("."), &export, false)
        .expect("the export succeeds");
    assert_eq!(
        String::from_utf8(data).expect("UTF-8"),
        "[\n  {\"Name\": \"ж;\"}\n]\n"
    );
}

#[test]
fn choosing_json_disables_the_csv_only_controls_and_reports_the_format() {
    let format = Rc::new(RefCell::new(ExportFormat::Csv));
    let options = Rc::new(RefCell::new(CsvConfiguration::default()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let mut harness = panel_harness_with_format(&format, &options, &actions);
    harness.run();

    harness
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Format")
        .click();
    harness.run();
    harness.get_by_label("JSON").click();
    harness.run();

    assert_eq!(*format.borrow(), ExportFormat::Json);
    assert_eq!(
        export::file_extension(*format.borrow(), &options.borrow()),
        "json"
    );
    for label in ["Encoding", "Separator", "Quote"] {
        assert!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::ComboBox, label)
                .accesskit_node()
                .is_disabled(),
            "{label} is CSV-only"
        );
    }
    assert!(
        !harness
            .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Decimal")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        harness
            .query_by_label("Column names become keys. The file is written in UTF-8.")
            .is_some()
    );

    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Export")
        .click();
    harness.run();
    assert!(matches!(
        actions.borrow().as_slice(),
        [ExportPanelAction::Export(ExportFormat::Json, _)]
    ));
}

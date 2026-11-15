//! Ports of the Apple `CSVClipboardTests` suite, plus the Review Focus cases
//! for trailing row breaks, inner tabs and line breaks, and numeric-only
//! localization.

use tabtool_core::{make_tsv, parse_tsv};

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect()
}

#[test]
fn parse_tsv_handles_crlf_and_trailing_newline() {
    assert_eq!(
        parse_tsv("a\tb\r\nc\td\r\n"),
        rows(&[&["a", "b"], &["c", "d"]])
    );
}

#[test]
fn parse_tsv_keeps_empty_fields_and_blank_rows() {
    assert_eq!(parse_tsv("a\t\tc\n\n"), rows(&[&["a", "", "c"], &[""]]));
}

#[test]
fn parse_tsv_handles_crlf_mixed_with_other_breaks() {
    assert_eq!(
        parse_tsv("a\tb\rc\td\ne\tf"),
        rows(&[&["a", "b"], &["c", "d"], &["e", "f"]])
    );
    assert_eq!(
        parse_tsv("a\r\nb\rc\nd"),
        rows(&[&["a"], &["b"], &["c"], &["d"]])
    );
}

#[test]
fn parse_tsv_drops_exactly_one_empty_row_from_trailing_newline() {
    assert_eq!(parse_tsv("a\n"), rows(&[&["a"]]));
    assert_eq!(parse_tsv("a\n\n"), rows(&[&["a"], &[""]]));
    assert_eq!(parse_tsv("a\n\n\n"), rows(&[&["a"], &[""], &[""]]));
    assert_eq!(parse_tsv("\n"), rows(&[&[""]]));
}

#[test]
fn parse_tsv_preserves_empty_fields_at_edges_and_middle() {
    assert_eq!(parse_tsv("\ta\t\t"), rows(&[&["", "a", "", ""]]));
    assert_eq!(parse_tsv("\t\t"), rows(&[&["", "", ""]]));
}

#[test]
fn parse_tsv_empty_input_returns_one_empty_field() {
    assert_eq!(parse_tsv(""), rows(&[&[""]]));
}

#[test]
fn parse_tsv_does_not_trim_or_interpret_quotes() {
    assert_eq!(parse_tsv(" a \t\"b\tc\""), rows(&[&[" a ", "\"b", "c\""]]));
}

#[test]
fn make_tsv_replaces_tabs_and_newlines_in_cells() {
    assert_eq!(make_tsv(&rows(&[&["a\tb", "c\nd"]]), ".", "."), "a b\tc d");
    assert_eq!(make_tsv(&rows(&[&["a\rb"]]), ".", "."), "a b");
    assert_eq!(make_tsv(&rows(&[&["a\r\nb"]]), ".", "."), "a b");
}

#[test]
fn make_tsv_localizes_numeric_cells() {
    assert_eq!(make_tsv(&rows(&[&["1,5", "x"]]), ",", "."), "1.5\tx");
}

#[test]
fn make_tsv_uses_requested_locale_for_localization() {
    assert_eq!(make_tsv(&rows(&[&["1.5", "2"]]), ".", ","), "1,5\t2");
}

#[test]
fn make_tsv_leaves_non_numeric_text_unchanged() {
    assert_eq!(
        make_tsv(&rows(&[&["a,5", "1,2,3"]]), ",", "."),
        "a,5\t1,2,3"
    );
}

#[test]
fn make_tsv_joins_rows_with_line_feeds_and_no_trailing_newline() {
    assert_eq!(
        make_tsv(&rows(&[&["a", "b"], &["c", "d"]]), ".", "."),
        "a\tb\nc\td"
    );
}

#[test]
fn make_tsv_preserves_empty_cells_and_rows() {
    assert_eq!(make_tsv(&rows(&[&["", "a"], &[""]]), ".", "."), "\ta\n");
    assert_eq!(make_tsv(&[], ".", "."), "");
}

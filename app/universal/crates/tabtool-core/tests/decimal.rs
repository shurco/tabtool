//! Ports of the Apple `DecimalMarkConverterTests` suite, plus the Review Focus
//! cases for no-op marks and paste-time conversion.

use tabtool_core::{convert_decimal_mark, parse_tsv};

#[test]
fn convert_only_numeric_cells() {
    assert_eq!(convert_decimal_mark("1,50", ",", "."), "1.50");
    assert_eq!(convert_decimal_mark("a,b", ",", "."), "a,b");
}

#[test]
fn convert_handles_signs_whitespace_and_exponent() {
    assert_eq!(convert_decimal_mark(" -1,5e3 ", ",", "."), " -1.5e3 ");
    assert_eq!(convert_decimal_mark("+0,25", ",", "."), "+0.25");
}

#[test]
fn convert_leaves_other_text_untouched() {
    assert_eq!(convert_decimal_mark("1,2.3", ",", "."), "1,2.3");
    assert_eq!(convert_decimal_mark("", ",", "."), "");
    assert_eq!(convert_decimal_mark("one,two", ",", "."), "one,two");
}

#[test]
fn convert_with_empty_source_mark_is_no_op() {
    assert_eq!(convert_decimal_mark("1.5", "", ","), "1.5");
    assert_eq!(convert_decimal_mark("abc", "", ","), "abc");
}

#[test]
fn convert_with_empty_target_mark_strips_the_mark() {
    assert_eq!(convert_decimal_mark("1.5", ".", ""), "15");
}

#[test]
fn convert_with_identical_marks_is_no_op() {
    assert_eq!(convert_decimal_mark("1.5", ".", "."), "1.5");
}

#[test]
fn paste_converts_the_locale_mark_only_for_numeric_cells() {
    let pasted = parse_tsv("1,5\ttext,5\r\n2,345\t1,2,3");
    let converted: Vec<Vec<String>> = pasted
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| convert_decimal_mark(cell, ",", "."))
                .collect()
        })
        .collect();

    assert_eq!(
        converted,
        vec![
            vec!["1.5".to_owned(), "text,5".to_owned()],
            vec!["2.345".to_owned(), "1,2,3".to_owned()],
        ]
    );
}

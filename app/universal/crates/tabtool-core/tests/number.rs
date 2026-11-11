//! Ports of the Apple `CSVNumberTests` suite, plus the Review Focus cases and
//! scanner-versus-regex parity checks.
//!
//! The Apple tests reach the internal scanner and regex directly through
//! `@testable import`. Integration tests cannot do that, so the scanner path is
//! pinned by the unit tests in `src/number.rs`, and the parity loop below
//! compares the public result (scanner first, regex fallback) with an
//! independently compiled reference regex.

use regex::Regex;
use tabtool_core::{is_numeric, localized};

/// The Apple suite's seeded generator, reproduced bit for bit so the random
/// inputs are the same.
struct SeededGenerator {
    state: u64,
}

impl SeededGenerator {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

/// The Apple pattern, compiled independently of the implementation.
fn reference_regex(decimal_mark: &str) -> Regex {
    let mark = regex::escape(decimal_mark);
    let pattern = format!(r"^\s*[+-]?(\d+{mark}?\d*|\d*{mark}?\d+)([eE][+-]?\d+)?\s*$");
    Regex::new(&pattern).expect("reference pattern compiles")
}

fn reference_is_numeric(text: &str, decimal_mark: &str) -> bool {
    reference_regex(decimal_mark).is_match(text)
}

#[test]
fn classifies_numeric_cells_like_swift() {
    let cases = [
        ("1", ".", true),
        ("1.5", ".", true),
        ("-1.5", ".", true),
        ("+1e3", ".", true),
        (" 42 ", ".", true),
        ("001", ".", true),
        ("1,5", ".", false),
        ("1.2.3", ".", false),
        ("", ".", false),
        ("1,5", ",", true),
        ("1.5", ",", false),
        ("1 2", ".", false),
        // Review Focus cases.
        (" 1.5 ", ".", true),
        ("+5", ".", true),
        ("-0", ".", true),
        ("001.50", ".", true),
        ("1e3", ".", true),
        ("1E-3", ".", true),
        ("1.", ".", true),
        (".5", ".", true),
    ];

    for (text, mark, expected) in cases {
        assert_eq!(is_numeric(text, mark), expected, "{text:?} with {mark:?}");
    }
}

#[test]
fn digit_and_e_marks_use_the_regex() {
    // The scanner refuses a mark that is a digit, a space or one of `eE+-`,
    // so these go through the regex, where the mark is optional and literal.
    assert!(is_numeric("5", "5"));
    assert!(!is_numeric("1.5", "5"));
    assert!(is_numeric("1e3", "e"));
    assert!(!is_numeric("1.5", "e"));
    // A multi-character mark is matched literally, so "ab?" requires the "a".
    assert!(!is_numeric("1", "ab"));
}

#[test]
fn localized_replaces_only_numeric_decimal_mark() {
    assert_eq!(localized("1,50", ",", "."), "1.50");
    assert_eq!(localized("hello,world", ",", "."), "hello,world");
    assert_eq!(localized("1.2.3", ".", ","), "1.2.3");
    assert_eq!(localized(" 1,5 ", ",", "."), " 1.5 ");
}

#[test]
fn localized_uses_the_given_separator() {
    assert_eq!(localized("1.50", ".", ","), "1,50");
    assert_eq!(localized("42", ".", ","), "42");
    // An absent separator leaves the text untouched.
    assert_eq!(localized("1,50", ",", ""), "1,50");
}

/// The scanner must agree with the regex on every ASCII input, including the
/// control characters and unusual shapes a fixed table would miss.
#[test]
fn numeric_agrees_with_reference_regex_on_random_ascii_text() {
    let alphabet: Vec<char> = "0123456789.,+-eE \t\n\r\u{0B}\u{0C}x".chars().collect();
    let marks = [".", ","];
    let references = marks.map(reference_regex);
    let mut generator = SeededGenerator::new(11);
    for _ in 0..50_000 {
        let length = (generator.next() % 8) as usize;
        let text: String = (0..length)
            .map(|_| alphabet[(generator.next() % alphabet.len() as u64) as usize])
            .collect();
        for (mark, reference) in marks.iter().zip(&references) {
            assert_eq!(
                is_numeric(&text, mark),
                reference.is_match(&text),
                "{text:?} with {mark:?}"
            );
        }
    }
}

#[test]
fn non_ascii_text_and_marks_use_the_unicode_regex() {
    // Non-ASCII input bypasses the ASCII scanner. The expected values are the
    // Unicode-aware reference regex; Task 12's Swift goldens pin the final
    // answers (Arabic-Indic digits included).
    let cases = [
        ("1\u{00A0}", "."),
        ("\u{0663}", "."),
        ("\u{0661}\u{0662}\u{0663}", "."),
        ("1.5", "ab"),
    ];

    for (text, mark) in cases {
        assert_eq!(
            is_numeric(text, mark),
            reference_is_numeric(text, mark),
            "{text:?} with {mark:?}"
        );
    }
}

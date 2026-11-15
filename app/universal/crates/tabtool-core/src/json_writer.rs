//! Writes a table as a JSON array with one object per row.
//!
//! Keys follow column order, and every value is a string: typing "007" or a
//! long ID as a number would lose data.

use std::fmt::Write as _;

use crate::{CsvTable, field_names};

/// The JSON text for `table`.
pub(crate) fn json_string(table: &CsvTable) -> String {
    let keys: Vec<String> = field_names::unique(field_names::names(table))
        .iter()
        .map(|key| quoted(key))
        .collect();
    let row_count = table.row_count();
    if row_count == 0 {
        return "[]\n".to_owned();
    }

    let mut output = String::from("[\n");
    for row in 0..row_count {
        let fields: Vec<String> = keys
            .iter()
            .enumerate()
            .map(|(column, key)| format!("{key}: {}", quoted(&table.cell(row, column))))
            .collect();
        output.push_str("  {");
        output.push_str(&fields.join(", "));
        output.push_str(if row + 1 == row_count { "}\n" } else { "},\n" });
    }
    output.push_str("]\n");
    output
}

/// A JSON string literal for `text` (RFC 8259 escaping).
fn quoted(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 2);
    result.push('"');
    for character in text.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{0}'..='\u{1F}' => {
                let _ = write!(result, "\\u{:04X}", u32::from(character));
            }
            _ => result.push(character),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::quoted;

    #[test]
    fn escapes_quotes_backslashes_and_control_characters() {
        assert_eq!(quoted("a\"b\\c/d"), r#""a\"b\\c/d""#);
        assert_eq!(quoted("1\n2\r3\t4"), r#""1\n2\r3\t4""#);
        assert_eq!(quoted("\u{0}\u{1F}"), r#""\u0000\u001F""#);
        assert_eq!(quoted("ж😀"), "\"ж😀\"");
    }
}

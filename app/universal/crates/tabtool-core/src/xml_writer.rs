//! Writes a table as an XML document: `<rows>` holds one `<row>` per table
//! row, and each cell is an element named after its column.

use std::fmt::Write as _;

use crate::{CsvTable, field_names};

/// The XML text for `table`.
pub(crate) fn xml_string(table: &CsvTable) -> String {
    let names = field_names::unique(
        field_names::names(table)
            .iter()
            .map(|name| element_name(name))
            .collect(),
    );
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rows>\n");
    for row in 0..table.row_count() {
        output.push_str("  <row>\n");
        for (column, name) in names.iter().enumerate() {
            let value = escaped(&table.cell(row, column));
            let _ = writeln!(output, "    <{name}>{value}</{name}>");
        }
        output.push_str("  </row>\n");
    }
    output.push_str("</rows>\n");
    output
}

/// A valid XML element name for a column name.
///
/// Characters outside the XML 1.0 `NameChar` production become `_`; the colon
/// is excluded too because it means a namespace prefix. A name that cannot
/// start an element (digit, `-`, `.`) or starts with the reserved `xml` gets a
/// leading `_`.
fn element_name(name: &str) -> String {
    let body: String = name
        .chars()
        .map(|character| {
            if is_name_char(character) {
                character
            } else {
                '_'
            }
        })
        .collect();
    let starts_well = body.chars().next().is_some_and(is_name_start);
    if starts_well && !body.to_lowercase().starts_with("xml") {
        body
    } else {
        format!("_{body}")
    }
}

/// XML 1.0 (Fifth Edition) `NameStartChar`, without the colon.
fn is_name_start(character: char) -> bool {
    matches!(
        u32::from(character),
        0x41..=0x5A
            | 0x5F
            | 0x61..=0x7A
            | 0xC0..=0xD6
            | 0xD8..=0xF6
            | 0xF8..=0x2FF
            | 0x370..=0x37D
            | 0x37F..=0x1FFF
            | 0x200C..=0x200D
            | 0x2070..=0x218F
            | 0x2C00..=0x2FEF
            | 0x3001..=0xD7FF
            | 0xF900..=0xFDCF
            | 0xFDF0..=0xFFFD
            | 0x10000..=0xEFFFF
    )
}

/// XML 1.0 (Fifth Edition) `NameChar`, without the colon.
fn is_name_char(character: char) -> bool {
    matches!(
        u32::from(character),
        0x2D | 0x2E | 0x30..=0x39 | 0xB7 | 0x300..=0x36F | 0x203F..=0x2040
    ) || is_name_start(character)
}

/// Element text with markup characters escaped.
///
/// Carriage returns become `&#13;` so parsers do not normalize them away.
fn escaped(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\r' => result.push_str("&#13;"),
            '\t' | '\n' => result.push(character),
            // ponytail: XML 1.0 has no way to encode these, even as references;
            // replaced with U+FFFD. Switch to XML 1.1 if users need them kept.
            '\u{0}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}' => result.push('\u{FFFD}'),
            _ => result.push(character),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{element_name, escaped};

    #[test]
    fn sanitizes_element_names() {
        assert_eq!(element_name("Unit Price"), "Unit_Price");
        assert_eq!(element_name("2024"), "_2024");
        assert_eq!(element_name("-x"), "_-x");
        assert_eq!(element_name("xmlData"), "_xmlData");
        assert_eq!(element_name("a:b<c>"), "a_b_c_");
        assert_eq!(element_name("Цена.v-1"), "Цена.v-1");
        assert_eq!(element_name("m²"), "m_");
    }

    #[test]
    fn escapes_text_and_preserves_carriage_returns() {
        assert_eq!(escaped("a<b>&c\"'"), "a&lt;b&gt;&amp;c\"'");
        assert_eq!(escaped("1\r\n2\t"), "1&#13;\n2\t");
    }

    /// XML 1.0 cannot represent these characters at all, not even as
    /// character references.
    #[test]
    fn replaces_characters_xml_cannot_represent() {
        assert_eq!(
            escaped("a\u{0}b\u{1B}c\u{FFFE}"),
            "a\u{FFFD}b\u{FFFD}c\u{FFFD}"
        );
    }
}

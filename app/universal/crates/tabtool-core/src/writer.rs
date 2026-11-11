//! Serialization of a [`CsvTable`] back to CSV text and bytes.

use crate::encoding::UTF8_BOM;
use crate::{CsvConfiguration, CsvError, CsvTable, Encoding, is_ambiguous};

/// Serializes a [`CsvTable`] with an explicit header row and configuration.
///
/// The writer only formats text; numeric cells and decimal marks reach it as
/// plain strings. Fields follow the table's visual column order. Quoting is
/// enabled whenever [`CsvConfiguration::quote_character`] is not empty, and
/// every field is quoted when enabled. Rows are separated by a line feed and
/// the output has no trailing newline. The header row is written as provided,
/// so callers normally pass [`CsvTable::headers`] or `None` to match the
/// document's header toggle.
pub struct CsvWriter<'a> {
    /// The table to serialize.
    pub table: &'a CsvTable,
    /// Header row written before the body, or `None` for no header row.
    pub header: Option<&'a [String]>,
    /// The encoding, separators and quoting to write with.
    pub configuration: &'a CsvConfiguration,
    /// Whether `data()` prepends a UTF-8 byte order mark.
    pub include_utf8_bom: bool,
    /// Whether `string()` skips the ambiguity check.
    pub allow_ambiguous_output: bool,
}

impl<'a> CsvWriter<'a> {
    /// Creates a writer without a byte order mark and with validation enabled.
    pub fn new(
        table: &'a CsvTable,
        header: Option<&'a [String]>,
        configuration: &'a CsvConfiguration,
    ) -> Self {
        Self {
            table,
            header,
            configuration,
            include_utf8_bom: false,
            allow_ambiguous_output: false,
        }
    }

    /// Returns the serialized CSV text without a trailing newline.
    ///
    /// Fails with [`CsvError::AmbiguousOutput`] when quoting is disabled and a
    /// field contains the column separator or a line break, unless
    /// [`CsvWriter::allow_ambiguous_output`] is set.
    pub fn string(&self) -> Result<String, CsvError> {
        if !self.allow_ambiguous_output && is_ambiguous(self.table, self.header, self.configuration)
        {
            return Err(CsvError::AmbiguousOutput);
        }
        Ok(Self::serialized_text(
            self.table,
            self.header,
            self.configuration,
        ))
    }

    /// Encodes [`CsvWriter::string`] with the configuration's encoding.
    ///
    /// A UTF-8 byte order mark is prepended when requested and the encoding is
    /// UTF-8. Fails with [`CsvError::UnencodableText`] when the output cannot
    /// be represented in the configured encoding; no bytes are returned then.
    pub fn data(&self) -> Result<Vec<u8>, CsvError> {
        let text = self.string()?;
        let encoded = self.configuration.encoding.encode(&text)?;
        if !self.include_utf8_bom || self.configuration.encoding != Encoding::Utf8 {
            return Ok(encoded);
        }

        let mut data = Vec::with_capacity(UTF8_BOM.len() + encoded.len());
        data.extend_from_slice(&UTF8_BOM);
        data.extend_from_slice(&encoded);
        Ok(data)
    }

    /// Serializes without ambiguity enforcement, for validation to inspect the
    /// exact text that saving would encode.
    pub fn serialized_text(
        table: &CsvTable,
        header: Option<&[String]>,
        configuration: &CsvConfiguration,
    ) -> String {
        let mut text = String::new();
        let mut is_first_row = true;

        if let Some(header) = header {
            append_row(
                header.iter().map(String::as_str),
                &mut text,
                &mut is_first_row,
                configuration,
            );
        }
        for row in 0..table.row_count() {
            append_row(
                table
                    .columns
                    .iter()
                    .map(|column| column.cells[row].as_str()),
                &mut text,
                &mut is_first_row,
                configuration,
            );
        }
        text
    }
}

/// Appends one row, separating it from a previous row with a line feed.
fn append_row<'fields>(
    fields: impl IntoIterator<Item = &'fields str>,
    text: &mut String,
    is_first_row: &mut bool,
    configuration: &CsvConfiguration,
) {
    if !*is_first_row {
        text.push('\n');
    }
    *is_first_row = false;

    for (index, field) in fields.into_iter().enumerate() {
        if index > 0 {
            text.push_str(&configuration.column_separator);
        }
        append_field(field, text, configuration);
    }
}

/// Appends one field, quoting and escaping it when quoting is enabled.
fn append_field(field: &str, text: &mut String, configuration: &CsvConfiguration) {
    let quote = configuration.quote_character.as_str();
    if quote.is_empty() {
        text.push_str(field);
        return;
    }

    // Mirror the parser: a dedicated escape character is doubled so it stays
    // literal, and a quote is prefixed with that escape. When the quote is its
    // own escape, the quote is doubled instead.
    let escape = configuration.escape_character.as_str();

    // Scanning for the first byte of each marker is a cheap superset test;
    // only a field that may contain one pays for the replacements.
    let quote_byte = quote.as_bytes().first().copied();
    let escape_byte = escape.as_bytes().first().copied();
    let may_escape = field
        .as_bytes()
        .iter()
        .any(|&byte| quote_byte == Some(byte) || escape_byte == Some(byte));
    if !may_escape {
        text.push_str(quote);
        text.push_str(field);
        text.push_str(quote);
        return;
    }

    let escaped = if escape == quote {
        field.replace(quote, &format!("{quote}{quote}"))
    } else {
        let mut escaped = field.to_owned();
        if !escape.is_empty() {
            escaped = escaped.replace(escape, &format!("{escape}{escape}"));
        }
        escaped.replace(quote, &format!("{escape}{quote}"))
    };

    text.push_str(quote);
    text.push_str(&escaped);
    text.push_str(quote);
}

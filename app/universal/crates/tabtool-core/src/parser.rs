//! Row-by-row reading of CSV text using a [`CsvConfiguration`].
//!
//! The parser scans an index over the decoded text and keeps its position until
//! [`CsvParser::reset`] is called. It returns cell text without classifying or
//! converting values; callers interpret the strings.

use crate::encoding::UTF8_BOM;
use crate::{CsvConfiguration, CsvError, Encoding};

/// Reads CSV text row by row using a [`CsvConfiguration`].
///
/// A leading UTF-8 byte order mark is removed from byte input and reported by
/// [`CsvParser::had_utf8_bom`]. Invalid bytes for the configured encoding fail
/// with [`CsvError::CannotDecodeData`].
pub struct CsvParser {
    scalars: Vec<char>,
    column_separator: Option<char>,
    quote_character: Option<char>,
    escape_character: Option<char>,
    index: usize,
    has_returned_row: bool,
    had_utf8_bom: bool,
}

impl CsvParser {
    /// Decodes `data` with the configuration's encoding and prepares a parser.
    ///
    /// The separator, quote and escape characters are the first scalar of
    /// their configuration strings, or absent when a string is empty.
    pub fn from_bytes(data: &[u8], configuration: &CsvConfiguration) -> Result<Self, CsvError> {
        let had_utf8_bom = configuration.encoding == Encoding::Utf8 && data.starts_with(&UTF8_BOM);
        let text = configuration.encoding.decode(data)?;
        Ok(Self::new(
            text.chars().collect(),
            configuration,
            had_utf8_bom,
        ))
    }

    /// Prepares a parser over already decoded text.
    pub fn from_string(text: &str, configuration: &CsvConfiguration) -> Self {
        Self::new(text.chars().collect(), configuration, false)
    }

    fn new(scalars: Vec<char>, configuration: &CsvConfiguration, had_utf8_bom: bool) -> Self {
        Self {
            scalars,
            column_separator: first_scalar(&configuration.column_separator),
            quote_character: first_scalar(&configuration.quote_character),
            escape_character: first_scalar(&configuration.escape_character),
            index: 0,
            has_returned_row: false,
            had_utf8_bom,
        }
    }

    /// Whether the input data began with a UTF-8 byte order mark.
    pub fn had_utf8_bom(&self) -> bool {
        self.had_utf8_bom
    }

    /// Whether all rows have been consumed.
    pub fn is_at_end(&self) -> bool {
        self.has_returned_row && self.index >= self.scalars.len()
    }

    /// Restarts reading from the beginning of the input.
    pub fn reset(&mut self) {
        self.index = 0;
        self.has_returned_row = false;
    }

    /// Returns the next row, or `None` when the input is exhausted.
    ///
    /// Fails with [`CsvError::MalformedQuotedField`] for an unterminated quoted
    /// field, a trailing escape, or content after a closing quote, and with
    /// [`CsvError::QuoteInUnquotedField`] when a quote appears in unquoted
    /// content while quotes are enabled.
    pub fn next_row(&mut self) -> Result<Option<Vec<String>>, CsvError> {
        if self.is_at_end() {
            return Ok(None);
        }
        if self.index >= self.scalars.len() {
            self.has_returned_row = true;
            return Ok(Some(vec![String::new()]));
        }

        let mut row = Vec::new();
        loop {
            row.push(self.read_field()?);
            if self.index == self.scalars.len() {
                self.has_returned_row = true;
                return Ok(Some(row));
            }
            if is_row_break(self.scalars[self.index]) {
                self.consume_row_break();
                self.has_returned_row = true;
                return Ok(Some(row));
            }
            // A field scan stops only at the separator or a row break.
            self.index += 1;
            if self.index == self.scalars.len() {
                row.push(String::new());
                self.has_returned_row = true;
                return Ok(Some(row));
            }
        }
    }

    /// Reads all remaining rows.
    pub fn read_all(&mut self) -> Result<Vec<Vec<String>>, CsvError> {
        let mut rows = Vec::new();
        while let Some(row) = self.next_row()? {
            rows.push(row);
        }
        Ok(rows)
    }

    fn read_field(&mut self) -> Result<String, CsvError> {
        if let Some(quote) = self.quote_character
            && self.index < self.scalars.len()
            && self.scalars[self.index] == quote
        {
            return self.read_quoted_field(quote);
        }
        self.read_unquoted_field()
    }

    fn read_quoted_field(&mut self, quote: char) -> Result<String, CsvError> {
        self.index += 1;
        let mut field = String::new();

        while self.index < self.scalars.len() {
            let scalar = self.scalars[self.index];

            // A dedicated escape character consumes the next scalar literally.
            if let Some(escape) = self.escape_character
                && escape != quote
                && scalar == escape
            {
                if self.index + 1 >= self.scalars.len() {
                    return Err(CsvError::MalformedQuotedField);
                }
                field.push(self.scalars[self.index + 1]);
                self.index += 2;
                continue;
            }

            if scalar != quote {
                field.push(scalar);
                self.index += 1;
                continue;
            }

            // With the quote as its own escape, a doubled quote is one quote.
            if self.escape_character == Some(quote)
                && self.index + 1 < self.scalars.len()
                && self.scalars[self.index + 1] == quote
            {
                field.push(quote);
                self.index += 2;
                continue;
            }

            self.index += 1;
            if self.index == self.scalars.len()
                || self.is_field_terminator(self.scalars[self.index])
            {
                return Ok(field);
            }
            return Err(CsvError::MalformedQuotedField);
        }

        Err(CsvError::MalformedQuotedField)
    }

    fn read_unquoted_field(&mut self) -> Result<String, CsvError> {
        let mut field = String::new();
        while self.index < self.scalars.len() && !self.is_field_terminator(self.scalars[self.index])
        {
            if self.quote_character == Some(self.scalars[self.index]) {
                return Err(CsvError::QuoteInUnquotedField);
            }
            field.push(self.scalars[self.index]);
            self.index += 1;
        }
        Ok(field)
    }

    fn is_field_terminator(&self, scalar: char) -> bool {
        self.column_separator == Some(scalar) || is_row_break(scalar)
    }

    fn consume_row_break(&mut self) {
        if self.scalars[self.index] == '\r'
            && self.index + 1 < self.scalars.len()
            && self.scalars[self.index + 1] == '\n'
        {
            self.index += 2;
        } else {
            self.index += 1;
        }
    }
}

/// The first scalar of a configuration string, or `None` when it is empty.
fn first_scalar(text: &str) -> Option<char> {
    text.chars().next()
}

/// Whether a scalar ends a row: LF, CR, NEL, line separator or paragraph
/// separator.
fn is_row_break(scalar: char) -> bool {
    matches!(
        scalar,
        '\u{000A}' | '\u{000D}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

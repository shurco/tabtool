//! Decoding of one index block into rows of text.

use tabtool_core::{CsvConfiguration, CsvParser, Encoding};

/// The decoded rows of one index block.
#[derive(Clone, Debug)]
pub struct LargeBlock {
    /// The rows of the block, without their line terminators.
    pub rows: Vec<Vec<String>>,
    /// False when the bytes were not valid text or did not parse: the rows are
    /// then a best-effort view and must not be rewritten.
    pub is_faithful: bool,
}

impl LargeBlock {
    /// Decodes the bytes of a block with [`CsvParser`].
    ///
    /// Invalid bytes show as U+FFFD. A parse error shows each raw line as one
    /// cell. Both lossy paths leave [`LargeBlock::is_faithful`] false.
    pub fn decode(bytes: &[u8], configuration: &CsvConfiguration) -> Self {
        if let Ok(mut parser) = CsvParser::from_bytes(bytes, configuration) {
            return match parser.read_all() {
                Ok(rows) => Self {
                    rows,
                    is_faithful: true,
                },
                Err(_) => Self {
                    rows: raw_lines(&lossy_text(bytes, configuration)),
                    is_faithful: false,
                },
            };
        }
        Self {
            rows: parse_lossy(bytes, configuration),
            is_faithful: false,
        }
    }

    /// Decodes a block and checks its row count against the index.
    ///
    /// The scanner ends rows only at LF, CR and CRLF, while [`CsvParser`] also
    /// treats NEL (U+0085), LS (U+2028) and PS (U+2029) as row breaks. A block
    /// with one of those characters outside quotes therefore decodes into more
    /// rows than the index recorded, and its rows can no longer be mapped to
    /// physical rows. Such a block is returned empty and unfaithful.
    pub(crate) fn decode_indexed(
        bytes: &[u8],
        configuration: &CsvConfiguration,
        expected_rows: usize,
    ) -> Self {
        let block = Self::decode(bytes, configuration);
        if block.rows.len() == expected_rows {
            block
        } else {
            Self {
                rows: Vec::new(),
                is_faithful: false,
            }
        }
    }
}

/// Text for display: U+FFFD for invalid UTF-8; other encodings fall back to
/// Latin-1, which accepts every byte.
fn lossy_text(bytes: &[u8], configuration: &CsvConfiguration) -> String {
    if configuration.encoding == Encoding::Utf8 {
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        bytes.iter().map(|&byte| char::from(byte)).collect()
    }
}

/// Parses the lossy text, falling back to one cell per raw line.
fn parse_lossy(bytes: &[u8], configuration: &CsvConfiguration) -> Vec<Vec<String>> {
    let text = lossy_text(bytes, configuration);
    CsvParser::from_string(&text, configuration)
        .read_all()
        .unwrap_or_else(|_| raw_lines(&text))
}

/// One cell per line, splitting on the newline characters Swift's
/// `Character.isNewline` recognizes; CR LF counts as one line break.
fn raw_lines(text: &str) -> Vec<Vec<String>> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                lines.push(vec![std::mem::take(&mut line)]);
            }
            '\n' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => {
                lines.push(vec![std::mem::take(&mut line)]);
            }
            _ => line.push(character),
        }
    }
    lines.push(vec![line]);
    if lines.len() > 1 && lines.last() == Some(&vec![String::new()]) {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_lines_treats_crlf_as_one_break() {
        assert_eq!(
            raw_lines("a\r\nb\rc\nd"),
            vec![vec!["a"], vec!["b"], vec!["c"], vec!["d"]]
        );
        assert_eq!(raw_lines("a\n"), vec![vec!["a"]]);
        assert_eq!(raw_lines(""), Vec::<Vec<String>>::new());
        assert_eq!(raw_lines("a\u{2028}b"), vec![vec!["a"], vec!["b"]]);
    }

    #[test]
    fn decode_indexed_rejects_a_row_count_mismatch() {
        let configuration = CsvConfiguration::default();
        let block = LargeBlock::decode_indexed("a\u{2028}b,c\n".as_bytes(), &configuration, 1);
        assert!(!block.is_faithful);
        assert!(block.rows.is_empty(), "an unaligned block maps no rows");

        let block = LargeBlock::decode_indexed(b"a,b\n", &configuration, 1);
        assert!(block.is_faithful);
        assert_eq!(block.rows, vec![vec!["a".to_string(), "b".to_string()]]);
    }
}

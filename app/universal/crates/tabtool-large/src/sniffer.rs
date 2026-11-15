//! Detects the format of a large file from its head and checks that the row
//! scanner can handle it.
//!
//! The scanner finds rows by looking at single bytes, which is only sound for
//! ASCII-compatible encodings and ASCII delimiters. Everything else is refused
//! with a [`CsvError`] instead of being indexed wrongly.

use tabtool_core::{CsvConfiguration, CsvError, CsvFormatDetector, CsvParser, Encoding};

use crate::{LargeFile, MappedFile};

/// Encodings whose rows can be found by scanning bytes.
///
/// GBK is included because no byte of a multi-byte GBK or GB18030 character is
/// a row break, so rows stay findable even though other ASCII bytes may appear
/// inside a character.
const SCANNABLE_ENCODINGS: [Encoding; 9] = [
    Encoding::Utf8,
    Encoding::MacRoman,
    Encoding::Windows1252,
    Encoding::IsoLatin2,
    Encoding::Windows1250,
    Encoding::Windows1251,
    Encoding::Windows1253,
    Encoding::Windows1254,
    Encoding::Gbk,
];

/// Byte prefixes that mark a UTF-16 or UTF-32 byte order mark.
const REFUSED_PREFIXES: [&[u8]; 3] = [&[0xFF, 0xFE], &[0xFE, 0xFF], &[0x00, 0x00, 0xFE, 0xFF]];

/// Whether rows of a file in `encoding` can be found by scanning bytes.
pub fn is_byte_scannable(encoding: Encoding) -> bool {
    SCANNABLE_ENCODINGS.contains(&encoding)
}

/// Returns an error when the scanner cannot handle `configuration`: a refused
/// encoding, or a separator, quote or escape that is not one ASCII character
/// (an empty string means the character is absent and is accepted).
pub fn validate(configuration: &CsvConfiguration) -> Result<(), CsvError> {
    if !is_byte_scannable(configuration.encoding) {
        return Err(CsvError::UnsupportedLargeFileEncoding);
    }
    let delimiters = [
        &configuration.column_separator,
        &configuration.quote_character,
        &configuration.escape_character,
    ];
    if delimiters
        .iter()
        .any(|delimiter| !is_single_ascii_byte(delimiter))
    {
        return Err(CsvError::UnsupportedLargeFileDelimiter);
    }
    Ok(())
}

/// Detects the configuration from the first [`LargeFile::SAMPLE_BYTES`] bytes.
///
/// The sample ends at the last row break outside quotes, so no row is cut in
/// half. It must parse strictly; the parser's error is returned otherwise. A
/// single row longer than the sample is parsed as is.
pub fn sniff(
    file: &MappedFile,
    prefers_chinese_encoding: bool,
) -> Result<CsvConfiguration, CsvError> {
    with_sample(file, |sample| {
        let configuration = CsvFormatDetector::new(sample, prefers_chinese_encoding).detect();
        verify(sample, &configuration)?;
        Ok(configuration)
    })
}

/// Checks that `configuration` suits the file: scannable, and the head of the
/// file parses strictly with it.
pub fn check(file: &MappedFile, configuration: &CsvConfiguration) -> Result<(), CsvError> {
    with_sample(file, |sample| verify(sample, configuration))
}

/// Whether `text` is at most one ASCII byte long.
fn is_single_ascii_byte(text: &str) -> bool {
    text.len() <= 1 && text.is_ascii()
}

/// Calls `body` with the sample of `file`, refusing encodings the byte scanner
/// cannot handle.
fn with_sample<R>(
    file: &MappedFile,
    body: impl FnOnce(&[u8]) -> Result<R, CsvError>,
) -> Result<R, CsvError> {
    // A positioned read never faults on pages a truncation removed; a short
    // read is the head of the file as it is now.
    let mut buffer = Vec::new();
    let read = file.read(0..LargeFile::SAMPLE_BYTES, &mut buffer, |head| head.len());
    let head = &buffer[..read];
    if looks_like_refused_encoding(head) {
        return Err(CsvError::UnsupportedLargeFileEncoding);
    }
    let sample = if file.size() > read {
        &head[..safe_cut(head)]
    } else {
        head
    };
    body(&sample[..trim_incomplete_utf8(sample)])
}

/// Strictly parses `sample` with `configuration`.
fn verify(sample: &[u8], configuration: &CsvConfiguration) -> Result<(), CsvError> {
    validate(configuration)?;
    CsvParser::from_bytes(sample, configuration)?.read_all()?;
    Ok(())
}

/// Whether `bytes` starts with a UTF-16/32 byte order mark, contains a NUL
/// byte, or carries an ISO-2022-JP escape sequence.
fn looks_like_refused_encoding(bytes: &[u8]) -> bool {
    if REFUSED_PREFIXES
        .iter()
        .any(|prefix| bytes.starts_with(prefix))
        || bytes.contains(&0)
    {
        return true;
    }
    bytes
        .windows(2)
        .any(|pair| pair[0] == 0x1B && matches!(pair[1], b'$' | b'('))
}

/// The length of `bytes` without an incomplete UTF-8 scalar at the end.
///
/// Only bytes that are valid UTF-8 except for a cut-off trailing scalar are
/// trimmed; invalid bytes elsewhere mean another encoding and stay untouched.
fn trim_incomplete_utf8(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Ok(_) => bytes.len(),
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => bytes.len(),
    }
}

/// The length of `bytes` up to the last row break with an even number of
/// quotes before it, or all of it when there is none.
fn safe_cut(bytes: &[u8]) -> usize {
    let mut quotes = 0_usize;
    let mut cut = None;
    for (offset, &byte) in bytes.iter().enumerate() {
        if byte == b'"' {
            quotes += 1;
        } else if matches!(byte, b'\n' | b'\r') && quotes.is_multiple_of(2) {
            cut = Some(offset + 1);
        }
    }
    cut.unwrap_or(bytes.len())
}

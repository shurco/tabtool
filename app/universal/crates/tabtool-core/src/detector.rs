//! Heuristic selection of a [`CsvConfiguration`] for raw data.

use std::borrow::Cow;

use crate::{CsvConfiguration, CsvParser, Encoding, is_numeric};

/// Picks the most likely [`CsvConfiguration`] for raw data.
///
/// The detector scores candidates in a fixed order:
///
/// - Eleven candidate configurations are scored in a fixed order.
/// - Each candidate parses at most five rows.
/// - Every parsed row adds one point.
/// - Rows that are not longer than the first row add their cell count.
/// - A candidate whose rows all have the same cell count adds five points and
///   marks the first row as a header.
/// - Numeric cells add one point each, up to five; a numeric or empty first-row
///   cell clears the header flag.
/// - A parse error skips the candidate; a decoding error ends the encoding
///   attempt.
///
/// Ties keep the earliest candidate, so the result is deterministic.
pub struct CsvFormatDetector<'a> {
    data: &'a [u8],
    prefers_chinese_encoding: bool,
}

/// Rows read per candidate configuration.
const SAMPLE_ROW_LIMIT: usize = 5;
/// Maximum number of numeric cells that add points.
const NUMBER_BONUS_LIMIT: i32 = 5;
/// Points for a candidate whose rows all have the same cell count.
const EQUAL_LENGTH_BONUS: i32 = 5;

impl<'a> CsvFormatDetector<'a> {
    /// Creates a detector for `data`.
    ///
    /// With `prefers_chinese_encoding`, GBK is tried before CP1252 and
    /// MacRoman.
    pub fn new(data: &'a [u8], prefers_chinese_encoding: bool) -> Self {
        Self {
            data,
            prefers_chinese_encoding,
        }
    }

    /// Returns the best configuration found by the heuristic.
    ///
    /// Detection never panics: a configuration is returned even when the data
    /// cannot be decoded or parsed with it, and the reader reports that error
    /// later.
    pub fn detect(&self) -> CsvConfiguration {
        let configurations = configurations();
        let encodings = encodings(self.prefers_chinese_encoding);
        let mut scores = [0_i32; 11];
        let mut header_flags = [false; 11];
        // The fallback mirrors the Apple detector's `encodings.last`.
        let mut detected_encoding = encodings.last().copied().unwrap_or(Encoding::Utf8);

        for encoding in encodings {
            // Every candidate shares the bytes, so decode once per encoding; a
            // decoding error ends the attempt and the next encoding is tried.
            let Some(text) = decode(self.data, encoding) else {
                continue;
            };
            for (index, template) in configurations.iter().enumerate() {
                let configuration = CsvConfiguration {
                    encoding,
                    ..template.clone()
                };
                let mut parser = CsvParser::from_string(&text, &configuration);
                let Some(rows) = read_rows(&mut parser) else {
                    continue;
                };
                scores[index] += rows.len() as i32;
                score_row_lengths(&rows, &mut scores[index], &mut header_flags[index]);
                score_numbers(
                    &rows,
                    &configuration.decimal_mark,
                    &mut scores[index],
                    &mut header_flags[index],
                );
            }
            detected_encoding = encoding;
            break;
        }

        // A strict comparison keeps the earlier candidate on a tie.
        let mut best_index = 0;
        for index in 1..scores.len() {
            if scores[index] > scores[best_index] {
                best_index = index;
            }
        }

        let mut best = configurations[best_index].clone();
        best.encoding = detected_encoding;
        best.first_row_as_header = header_flags[best_index];
        best
    }
}

/// The candidates in scoring order, which differs from the `config1`...`config11`
/// numbering: 1, 10, 6, 2, 5, 7, 3, 8, 4, 9, 11.
fn configurations() -> [CsvConfiguration; 11] {
    let default = CsvConfiguration::default;
    [
        default(),
        CsvConfiguration {
            escape_character: "\\".to_owned(),
            ..default()
        },
        CsvConfiguration {
            quote_character: String::new(),
            ..default()
        },
        CsvConfiguration {
            column_separator: ";".to_owned(),
            decimal_mark: ",".to_owned(),
            ..default()
        },
        CsvConfiguration {
            column_separator: ";".to_owned(),
            escape_character: "\\".to_owned(),
            decimal_mark: ",".to_owned(),
            ..default()
        },
        CsvConfiguration {
            column_separator: ";".to_owned(),
            quote_character: String::new(),
            decimal_mark: ",".to_owned(),
            ..default()
        },
        CsvConfiguration {
            column_separator: "\t".to_owned(),
            ..default()
        },
        CsvConfiguration {
            column_separator: "\t".to_owned(),
            quote_character: String::new(),
            ..default()
        },
        CsvConfiguration {
            column_separator: "\t".to_owned(),
            decimal_mark: ",".to_owned(),
            ..default()
        },
        CsvConfiguration {
            column_separator: "\t".to_owned(),
            quote_character: String::new(),
            decimal_mark: ",".to_owned(),
            ..default()
        },
        CsvConfiguration {
            decimal_mark: ",".to_owned(),
            ..default()
        },
    ]
}

/// The encoding attempts, GBK second when Chinese is preferred.
fn encodings(prefers_chinese_encoding: bool) -> [Encoding; 4] {
    if prefers_chinese_encoding {
        [
            Encoding::Utf8,
            Encoding::Gbk,
            Encoding::Windows1252,
            Encoding::MacRoman,
        ]
    } else {
        [
            Encoding::Utf8,
            Encoding::Windows1252,
            Encoding::MacRoman,
            Encoding::Gbk,
        ]
    }
}

/// Decodes `data` for one encoding attempt, or returns `None` when Apple's
/// Foundation decoder would reject the bytes.
fn decode(data: &[u8], encoding: Encoding) -> Option<Cow<'_, str>> {
    encoding.decode(data).ok()
}

/// Reads at most [`SAMPLE_ROW_LIMIT`] rows, or returns `None` on a parse error.
fn read_rows(parser: &mut CsvParser) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    while rows.len() < SAMPLE_ROW_LIMIT {
        match parser.next_row() {
            Ok(Some(row)) => rows.push(row),
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(rows)
}

fn score_row_lengths(rows: &[Vec<String>], score: &mut i32, header_flag: &mut bool) {
    let Some(first_row) = rows.first() else {
        return;
    };
    let first_row_cell_count = first_row.len();
    let mut all_rows_have_same_length = true;
    for row in &rows[1..] {
        all_rows_have_same_length &= row.len() == first_row_cell_count;
        if row.len() <= first_row_cell_count {
            *score += row.len() as i32;
        }
    }
    if all_rows_have_same_length {
        *score += EQUAL_LENGTH_BONUS;
        *header_flag = true;
    }
}

fn score_numbers(
    rows: &[Vec<String>],
    decimal_mark: &str,
    score: &mut i32,
    header_flag: &mut bool,
) {
    let mut number_count = 0;
    for (row_index, row) in rows.iter().enumerate() {
        for cell in row {
            if is_numeric(cell, decimal_mark) {
                number_count += 1;
                if row_index == 0 {
                    *header_flag = false;
                }
            } else if row_index == 0 && cell.is_empty() {
                *header_flag = false;
            }
        }
    }
    *score += number_count.min(NUMBER_BONUS_LIMIT);
}

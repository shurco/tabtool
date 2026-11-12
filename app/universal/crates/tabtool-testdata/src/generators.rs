//! Deterministic CSV generators ported from the Apple test support code.
//!
//! [`make_benchmark_csv`] mirrors `makeBenchmarkCSV` in `TestSupport.swift`.
//! [`generate_large_csv`] mirrors `generateLargeCSV` in
//! `LargeFileTestSupport.swift`, including its SplitMix64 stream, 8 MiB
//! chunks, CRLF discipline, BOM, quoted multiline rows and the `quoted`
//! switch for odd text fields.

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

/// Options for [`generate_large_csv`].
#[derive(Clone, Debug)]
pub struct LargeOptions {
    /// Destination file, created or truncated.
    pub path: PathBuf,
    /// Target size in bytes. Generation stops once at least this many bytes
    /// have been produced.
    pub bytes: u64,
    /// Fields per row. Must be greater than zero.
    pub columns: usize,
    /// Every Nth row (the header is row 1) gets a quoted multiline field.
    pub multiline_every: Option<usize>,
    /// End rows with CRLF instead of LF. Embedded multiline fields keep LF.
    pub crlf: bool,
    /// Prefix the file with a UTF-8 BOM.
    pub bom: bool,
    /// Quote odd text fields. Multiline fields stay quoted either way.
    pub quoted: bool,
    /// SplitMix64 seed.
    pub seed: u64,
}

/// Summary of a generated large CSV.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LargeFixture {
    /// Path that was written.
    pub path: PathBuf,
    /// Rows written, header included.
    pub rows: u64,
    /// Bytes written, BOM included.
    pub bytes: u64,
}

/// SplitMix64, matching `SeededGenerator` in `TestSupport.swift`.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
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

/// Builds the deterministic benchmark CSV used by the Apple performance tests:
/// a header row, even columns holding numbers, odd columns holding text, and,
/// when `quoted`, a multiline quoted field on every tenth row.
pub fn make_benchmark_csv(rows: usize, columns: usize, quoted: bool) -> String {
    let mut generator = SplitMix64::new(42);
    let mut text = String::new();

    for column in 0..columns {
        if column > 0 {
            text.push(',');
        }
        text.push_str("col");
        text.push_str(&column.to_string());
    }
    text.push('\n');

    for row in 0..rows {
        for column in 0..columns {
            if column > 0 {
                text.push(',');
            }
            if column % 2 == 0 {
                let value = generator.next();
                text.push_str(&format!("{}.{}", value % 1_000_000, column));
            } else if quoted && row % 10 == 0 {
                text.push_str(&format!("\"line one\nline, \"\"two\"\" {row}\""));
            } else {
                let value = generator.next();
                text.push_str(&format!("text{}", value % 5000));
            }
        }
        text.push('\n');
    }

    text
}

/// Streams a deterministic CSV of at least `options.bytes` to `options.path`
/// and reports the rows (header included) and bytes written.
pub fn generate_large_csv(options: &LargeOptions) -> io::Result<LargeFixture> {
    assert!(options.columns > 0, "columns must be greater than zero");
    assert!(
        options.multiline_every != Some(0),
        "multiline_every must be greater than zero"
    );

    const CHUNK_LIMIT: usize = 8 << 20;

    fn flush(file: &mut File, chunk: &mut Vec<u8>, written: &mut u64) -> io::Result<()> {
        file.write_all(chunk)?;
        *written += chunk.len() as u64;
        chunk.clear();
        Ok(())
    }

    let terminator: &[u8] = if options.crlf { b"\r\n" } else { b"\n" };
    let multiline_column = 1.min(options.columns - 1);
    let mut file = File::create(&options.path)?;
    let mut generator = SplitMix64::new(options.seed);
    let mut chunk = Vec::with_capacity(CHUNK_LIMIT + 4_096);
    let mut written = 0_u64;
    let mut rows = 0_u64;

    if options.bom {
        chunk.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    for column in 0..options.columns {
        if column > 0 {
            chunk.push(b',');
        }
        write!(&mut chunk, "col{column}").expect("writing to a Vec cannot fail");
    }
    chunk.extend_from_slice(terminator);
    rows += 1;

    while written + (chunk.len() as u64) < options.bytes {
        for column in 0..options.columns {
            if column > 0 {
                chunk.push(b',');
            }
            let value = generator.next();
            let multiline = options.multiline_every.is_some_and(|every| {
                rows.is_multiple_of(every as u64) && column == multiline_column
            });
            if multiline {
                write!(&mut chunk, "\"line one\nline, \"\"two\"\" {rows}\"")
                    .expect("writing to a Vec cannot fail");
            } else if column % 2 == 0 {
                write!(&mut chunk, "{}.{}", value % 1_000_000, column)
                    .expect("writing to a Vec cannot fail");
            } else if options.quoted {
                write!(&mut chunk, "\"text{}\"", value % 5000)
                    .expect("writing to a Vec cannot fail");
            } else {
                write!(&mut chunk, "text{}", value % 5000).expect("writing to a Vec cannot fail");
            }
        }
        chunk.extend_from_slice(terminator);
        rows += 1;
        if chunk.len() >= CHUNK_LIMIT {
            flush(&mut file, &mut chunk, &mut written)?;
        }
    }
    flush(&mut file, &mut chunk, &mut written)?;

    Ok(LargeFixture {
        path: options.path.clone(),
        rows,
        bytes: written,
    })
}

//! Checks that a saved benchmark file holds the same bytes or the expected
//! edits.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tabtool_core::CsvConfiguration;
use tabtool_large::{CellKey, CellOverlay, LargeCSVSource, LargeFileIndexer, MappedFile};

use tabtool_testdata::SplitMix64;

const COMPARE_CHUNK_BYTES: usize = 4 << 20;
const INDEX_STEP_BYTES: usize = 8 << 20;
const SAMPLE_READS: usize = 10_000;

/// What a save verification checked.
pub struct Verification {
    bytes: Option<u64>,
    cells: usize,
    edits: usize,
}

impl Verification {
    /// A row value for the Markdown table.
    pub fn summary(&self) -> String {
        match self.bytes {
            Some(bytes) => format!("verified {bytes} bytes byte-identical"),
            None if self.edits == 1 => format!("verified 1 edit and {} sampled cells", self.cells),
            None => format!(
                "verified {} edits and {} sampled cells",
                self.edits, self.cells
            ),
        }
    }
}

/// Verifies a saved file: without edits it must be byte-identical; with edits
/// they must be present and 10k sampled rows must match the source.
pub fn saved(
    output: &Path,
    original: &Path,
    reference: &LargeCSVSource,
    configuration: &CsvConfiguration,
    edits: &HashMap<CellKey, String>,
    written: u64,
    random: &mut SplitMix64,
) -> Result<Verification, String> {
    let saved = fs::metadata(output)
        .map_err(|error| format!("cannot read {}: {error}", output.display()))?;
    if saved.len() != written {
        return Err(format!(
            "{} is {} bytes, but the save wrote {written}",
            output.display(),
            saved.len()
        ));
    }
    if edits.is_empty() {
        let bytes = identical_bytes(original, output)?;
        return Ok(Verification {
            bytes: Some(bytes),
            cells: 0,
            edits: 0,
        });
    }

    let file = Arc::new(
        MappedFile::open(output)
            .map_err(|error| format!("cannot reopen {}: {error}", output.display()))?,
    );
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&file),
        configuration.clone(),
        INDEX_STEP_BYTES,
        Duration::ZERO,
    ));
    indexer.start(|_| {});
    indexer.wait();
    let copy = LargeCSVSource::new(
        file,
        configuration.clone(),
        indexer,
        CellOverlay::new(),
        Vec::new(),
    );
    if copy.row_count() != reference.row_count() || copy.column_count() != reference.column_count()
    {
        return Err(format!(
            "{} holds {} rows x {} columns, expected {} x {}",
            output.display(),
            copy.row_count(),
            copy.column_count(),
            reference.row_count(),
            reference.column_count()
        ));
    }

    let header_rows = reference.physical_row(0);
    for (key, text) in edits {
        let row = key.row.checked_sub(header_rows).ok_or_else(|| {
            format!(
                "the edit at row {}, column {} is above the header",
                key.row, key.column
            )
        })?;
        let actual = copy.cell(row, key.column);
        if actual != *text {
            return Err(format!(
                "the edit at row {row}, column {} is {actual:?}, expected {text:?}",
                key.column
            ));
        }
    }

    let mut cells = 0;
    for _ in 0..SAMPLE_READS {
        let row = random.below(copy.row_count());
        for column in 0..copy.column_count() {
            let actual = copy.cell(row, column);
            let expected = reference.cell(row, column);
            if actual != expected {
                return Err(format!(
                    "row {row}, column {column} is {actual:?}, expected {expected:?}"
                ));
            }
            cells += 1;
        }
    }
    Ok(Verification {
        bytes: None,
        cells,
        edits: edits.len(),
    })
}

/// Compares two files in 4 MiB chunks and returns the bytes compared.
fn identical_bytes(original: &Path, output: &Path) -> Result<u64, String> {
    let mut original = File::open(original)
        .map_err(|error| format!("cannot open {}: {error}", original.display()))?;
    let mut output =
        File::open(output).map_err(|error| format!("cannot open {}: {error}", output.display()))?;
    let mut left = vec![0_u8; COMPARE_CHUNK_BYTES];
    let mut right = vec![0_u8; COMPARE_CHUNK_BYTES];
    let mut compared = 0_u64;
    loop {
        let left_len = read_chunk(&mut original, &mut left)?;
        let right_len = read_chunk(&mut output, &mut right)?;
        if left_len != right_len {
            return Err(format!(
                "the saved file differs in length at byte {compared}"
            ));
        }
        if left[..left_len] != right[..right_len] {
            return Err(format!("the saved bytes differ at byte {compared}"));
        }
        compared += left_len as u64;
        if left_len < COMPARE_CHUNK_BYTES {
            return Ok(compared);
        }
    }
}

/// Fills `buffer` unless the end of the file is reached, retrying interrupts.
fn read_chunk(file: &mut File, buffer: &mut [u8]) -> Result<usize, String> {
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("cannot read the saved bytes: {error}")),
        }
    }
    Ok(filled)
}

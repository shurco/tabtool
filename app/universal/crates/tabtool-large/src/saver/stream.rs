//! Streams a save snapshot into the output, rewriting only the edited rows.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use tabtool_core::{CsvError, CsvTable, CsvWriter};

use super::error::{LargeFileSaveError, cancelled_error};
use crate::LargeFile;
use crate::block::LargeBlock;
use crate::mapped::MappedFile;
use crate::scanner::row_ranges;
use crate::source::LargeSaveSnapshot;

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Streams the snapshot while tracking the cursor, the bytes written and the
/// progress reports.
struct Output<'a, P> {
    output: &'a mut File,
    buffer: Vec<u8>,
    buffer_bytes: usize,
    progress: &'a P,
    cancelled: &'a AtomicBool,
    written: u64,
}

impl<P: Fn(f64)> Output<'_, P> {
    fn emit(&mut self, bytes: &[u8]) -> Result<(), LargeFileSaveError> {
        self.output.write_all(bytes)?;
        self.written += bytes.len() as u64;
        Ok(())
    }

    /// Copies the unedited bytes from `cursor` up to `end`.
    fn copy(
        &mut self,
        file: &MappedFile,
        cursor: &mut usize,
        end: usize,
        size: usize,
    ) -> Result<(), LargeFileSaveError> {
        let Self {
            output,
            buffer,
            buffer_bytes,
            progress,
            cancelled,
            written,
        } = self;
        while *cursor < end {
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_error());
            }
            let next = cursor.saturating_add(*buffer_bytes).min(end);
            let chunk = *cursor..next;
            let wanted = chunk.len();
            let mut outcome = Ok(());
            file.read(chunk, buffer, |bytes| {
                if bytes.len() != wanted {
                    outcome = Err(LargeFileSaveError::SourceChanged);
                    return;
                }
                outcome = output.write_all(bytes).map_err(LargeFileSaveError::Io);
            });
            outcome?;
            *written += wanted as u64;
            *cursor = next;
            progress(*cursor as f64 / size.max(1) as f64);
        }
        Ok(())
    }
}

pub(super) fn stream(
    snapshot: &LargeSaveSnapshot,
    output: &mut File,
    buffer_bytes: usize,
    progress: &impl Fn(f64),
    cancelled: &AtomicBool,
) -> Result<u64, LargeFileSaveError> {
    let size = snapshot.file.size();
    let mut cursor = 0;
    let mut writer = Output {
        output,
        buffer: Vec::new(),
        buffer_bytes,
        progress,
        cancelled,
        written: 0,
    };
    let blocks = edits_by_block(snapshot);
    for (number, rows) in blocks {
        let block_range = snapshot
            .index
            .block_range(number, size)
            .ok_or(LargeFileSaveError::Csv(CsvError::RowNotEditable))?;
        if !snapshot.file.is_readable() {
            return Err(LargeFileSaveError::SourceChanged);
        }
        let mut block_bytes = Vec::new();
        let read = snapshot
            .file
            .read(block_range.clone(), &mut block_bytes, |bytes| bytes.len());
        if read != block_range.len() {
            return Err(LargeFileSaveError::SourceChanged);
        }
        block_bytes.truncate(read);
        let expected_rows = snapshot.index.block_row_count(number);
        let block =
            LargeBlock::decode_indexed(&block_bytes, &snapshot.configuration, expected_rows);
        let ranges = row_ranges(&block_bytes, &snapshot.configuration);
        if !block.is_faithful || ranges.len() != expected_rows {
            return Err(LargeFileSaveError::Csv(CsvError::RowNotEditable));
        }
        for (row, edits) in rows {
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_error());
            }
            let local = row % LargeFile::BLOCK_ROWS;
            let row_range = ranges
                .get(local)
                .ok_or(LargeFileSaveError::Csv(CsvError::RowNotEditable))?;
            writer.copy(
                &snapshot.file,
                &mut cursor,
                block_range.start + row_range.start,
                size,
            )?;
            let row_bytes = &block_bytes[row_range.clone()];
            writer.emit(&serialized_row(
                &block.rows[local],
                row,
                &edits,
                snapshot,
                row_bytes,
            )?)?;
            cursor = block_range.start + row_range.end;
        }
    }
    writer.copy(&snapshot.file, &mut cursor, size, size)?;
    (writer.progress)(1.0);
    Ok(writer.written)
}

/// The edited cells of one row: column and replacement text.
type RowEdits<'a> = Vec<(usize, &'a str)>;

/// The edited rows grouped by block number, then by row.
type BlockEdits<'a> = BTreeMap<usize, BTreeMap<usize, RowEdits<'a>>>;

/// The edited rows grouped by block, then by row, sorted so the cursor only
/// moves forward.
fn edits_by_block(snapshot: &LargeSaveSnapshot) -> BlockEdits<'_> {
    let mut blocks: BlockEdits<'_> = BTreeMap::new();
    for (key, text) in &snapshot.edits {
        blocks
            .entry(key.row / LargeFile::BLOCK_ROWS)
            .or_default()
            .entry(key.row)
            .or_default()
            .push((key.column, text.as_str()));
    }
    blocks
}

/// The edited row as bytes: patched fields, then the original terminator and,
/// for row 0, the BOM.
fn serialized_row(
    fields: &[String],
    row: usize,
    edits: &[(usize, &str)],
    snapshot: &LargeSaveSnapshot,
    row_bytes: &[u8],
) -> Result<Vec<u8>, LargeFileSaveError> {
    let table = CsvTable::from_rows(vec![patched_row(fields, edits)], None, 0);
    let body = CsvWriter::new(&table, None, &snapshot.configuration).data()?;
    let terminator = terminator(row_bytes);
    let mut data = Vec::with_capacity(UTF8_BOM.len() + body.len() + terminator.len());
    if row == 0 && snapshot.had_bom {
        data.extend_from_slice(&UTF8_BOM);
    }
    data.extend_from_slice(&body);
    data.extend_from_slice(terminator);
    Ok(data)
}

/// The fields with the edits applied, padded to the widest column the edits
/// address.
fn patched_row(fields: &[String], edits: &[(usize, &str)]) -> Vec<String> {
    let width = edits
        .iter()
        .map(|&(column, _)| column.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(fields.len());
    let mut patched = fields.to_vec();
    patched.resize(width, String::new());
    for &(column, text) in edits {
        if let Some(cell) = patched.get_mut(column) {
            *cell = text.to_string();
        }
    }
    patched
}

/// The trailing CR/LF bytes of a terminator-inclusive row range.
fn terminator(row: &[u8]) -> &[u8] {
    let mut start = row.len();
    while start > 0 && matches!(row[start - 1], b'\n' | b'\r') {
        start -= 1;
    }
    &row[start..]
}

//! Streaming rewrite of a large file with its edited rows.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use tabtool_core::{CsvError, CsvTable, CsvWriter};

use crate::LargeFile;
use crate::block::LargeBlock;
use crate::mapped::{MappedFile, Staleness};
use crate::platform;
use crate::scanner::row_ranges;
use crate::source::LargeSaveSnapshot;

/// Rows are copied in chunks of this size when nothing says otherwise.
const COPY_BUFFER_BYTES: usize = 4 * 1_048_576;
const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];
const TEMPORARY_DIRECTORY_PREFIX: &str = ".tabtool-save-";

/// Why a large file cannot be saved right now.
#[derive(Debug)]
pub enum LargeFileSaveError {
    /// The whole file has not been indexed yet.
    IndexingNotFinished,
    /// The file on disk is no longer the one that was mapped.
    SourceChanged,
    /// The volume holding the target has less than twice the file's size free.
    InsufficientDiskSpace,
    /// An edited row cannot be written with the file's format.
    Csv(CsvError),
    /// The target could not be written or replaced, or the save was cancelled
    /// ([`io::ErrorKind::Interrupted`]).
    Io(io::Error),
}

impl From<CsvError> for LargeFileSaveError {
    fn from(error: CsvError) -> Self {
        Self::Csv(error)
    }
}

impl From<io::Error> for LargeFileSaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Writes the saved file to `to` and returns its size in bytes.
///
/// Rows without edits are copied byte for byte in chunks of `buffer_bytes`
/// (at least one). An edited row is parsed from its block, patched and
/// serialized again with [`CsvWriter`], keeping its original line terminator
/// and, for row 0, the UTF-8 byte order mark.
///
/// `progress` receives the fraction done, ending at `1.0`. The write stops with
/// [`io::ErrorKind::Interrupted`] when `cancelled` is set. A failed or cancelled
/// write removes the output file; if `to` already existed, that file is removed
/// too. `to` must name a file other than the mapped source, including through a
/// hard link or symbolic link: the write fails with
/// [`io::ErrorKind::InvalidInput`] before truncating anything.
pub fn write(
    snapshot: &LargeSaveSnapshot,
    to: &Path,
    buffer_bytes: usize,
    progress: impl Fn(f64),
    cancelled: &AtomicBool,
) -> Result<u64, LargeFileSaveError> {
    let (output, written) =
        create_and_stream(snapshot, to, buffer_bytes.max(1), &progress, cancelled)?;
    drop(output);
    Ok(written)
}

/// Creates `to` and streams the snapshot into it.
///
/// Returns the open file so `save` can flush it through the descriptor that
/// wrote it, even when the process umask leaves the file unreadable by name.
/// A failed or cancelled stream removes the output file before returning.
fn create_and_stream(
    snapshot: &LargeSaveSnapshot,
    to: &Path,
    buffer_bytes: usize,
    progress: &impl Fn(f64),
    cancelled: &AtomicBool,
) -> Result<(File, u64), LargeFileSaveError> {
    ensure_ready(snapshot)?;
    if snapshot.file.is_same_file(to)? {
        return Err(LargeFileSaveError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the output is the mapped source file",
        )));
    }
    let mut output = File::create(to)?;
    match stream(snapshot, &mut output, buffer_bytes, progress, cancelled) {
        Ok(written) => Ok((output, written)),
        Err(error) => {
            // Close before removing: Windows rejects removing an open file.
            drop(output);
            let _ = fs::remove_file(to);
            Err(error)
        }
    }
}

/// Writes the saved file next to `replacing` and swaps it in.
///
/// Needs twice the file's size free on the volume; `available_capacity`
/// returns the free bytes for a path, and `None` skips the check. The file is
/// written into a private temporary directory next to the target, flushed to
/// disk through the descriptor that wrote it, and then swapped in with
/// [`platform::replace_file`]. The temporary directory is removed in every
/// case.
///
/// An existing target keeps its permissions (Unix); its other metadata is not
/// copied. On Unix the parent directory is synced after the swap.
///
/// The snapshot is consumed so its mapping is dropped before the swap: Windows
/// cannot replace a file that is still mapped. If the caller kept another
/// reference, the replacement fails with a typed I/O error; there is no
/// in-place fallback.
///
/// On Unix the parent directory is synced after the swap. If that sync fails,
/// the error is returned although `replacing` already holds the saved bytes;
/// `Err` does not always mean the old file is still on disk.
pub fn save(
    snapshot: LargeSaveSnapshot,
    replacing: &Path,
    available_capacity: impl Fn(&Path) -> Option<u64>,
    progress: impl Fn(f64),
    cancelled: &AtomicBool,
) -> Result<(), LargeFileSaveError> {
    if snapshot.index.scanned_bytes < snapshot.file.size() {
        return Err(LargeFileSaveError::IndexingNotFinished);
    }
    let required = u64::try_from(snapshot.file.size())
        .unwrap_or(u64::MAX)
        .saturating_mul(2);
    if let Some(free) = available_capacity(replacing)
        && free < required
    {
        return Err(LargeFileSaveError::InsufficientDiskSpace);
    }

    let parent = match replacing.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = replacing.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "the save target has no file name",
        )
    })?;
    let directory = tempfile::Builder::new()
        .prefix(TEMPORARY_DIRECTORY_PREFIX)
        .tempdir_in(parent)?;
    let temporary = directory.path().join(name);

    let (output, _) = create_and_stream(
        &snapshot,
        &temporary,
        COPY_BUFFER_BYTES,
        &progress,
        cancelled,
    )?;
    output.sync_all()?;
    drop(output);
    #[cfg(unix)]
    copy_permissions(replacing, &temporary)?;
    drop(snapshot);
    install(&temporary, replacing)?;
    #[cfg(unix)]
    sync_directory(parent)?;
    Ok(())
}

/// Free bytes available to the current user on the volume holding `path`.
///
/// A path that does not exist yet reports its volume through the nearest
/// existing ancestor, so a Save As target can be checked before it is created.
pub fn volume_capacity(path: &Path) -> Option<u64> {
    let mut candidate = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    loop {
        if let Ok(space) = platform::available_space(candidate) {
            return Some(space);
        }
        match candidate.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => candidate = parent,
            Some(_) if !candidate.is_absolute() && candidate != Path::new(".") => {
                candidate = Path::new(".");
            }
            _ => return None,
        }
    }
}

/// Moves `temporary` onto `target`, replacing an existing file atomically.
fn install(temporary: &Path, target: &Path) -> io::Result<()> {
    if target.try_exists()? {
        platform::replace_file(target, temporary)
    } else {
        // `ReplaceFileW` needs an existing target; a new file is a plain rename.
        fs::rename(temporary, target)
    }
}

/// Gives `target` the permissions of `source` when `source` exists.
///
/// A rename keeps the temporary file's mode (`0666` less the umask), so a
/// `0600` document would otherwise become world-readable after a save. A
/// missing source is a new file and keeps the default.
#[cfg(unix)]
fn copy_permissions(source: &Path, target: &Path) -> io::Result<()> {
    match fs::metadata(source) {
        Ok(metadata) => fs::set_permissions(target, metadata.permissions()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Flushes a directory entry change (a rename) to disk.
#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

fn ensure_ready(snapshot: &LargeSaveSnapshot) -> Result<(), LargeFileSaveError> {
    if snapshot.index.scanned_bytes < snapshot.file.size() {
        return Err(LargeFileSaveError::IndexingNotFinished);
    }
    if snapshot.file.is_stale() != Staleness::Unchanged {
        return Err(LargeFileSaveError::SourceChanged);
    }
    Ok(())
}

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

fn stream(
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

fn cancelled_error() -> LargeFileSaveError {
    LargeFileSaveError::Io(io::Error::new(io::ErrorKind::Interrupted, "save cancelled"))
}

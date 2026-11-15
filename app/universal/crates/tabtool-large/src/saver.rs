//! Streaming rewrite of a large file with its edited rows.

mod error;
mod stream;

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub use error::LargeFileSaveError;
use error::cancelled_error;
use stream::stream;

use crate::mapped::Staleness;
use crate::platform;
use crate::source::LargeSaveSnapshot;

/// Rows are copied in chunks of this size when nothing says otherwise.
const COPY_BUFFER_BYTES: usize = 4 * 1_048_576;

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
///
/// [`CsvWriter`]: tabtool_core::CsvWriter
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
/// On Unix the parent directory is synced after the swap. A failing sync is
/// reported to the error stream and does not fail the save: the target already
/// holds the saved bytes, so the caller must adopt them instead of restoring
/// the old state.
///
/// A caller that needs to drop its own mapping between writing and replacing
/// uses [`LargeFileSaver::prepare`] and [`PreparedSave::install`] instead.
pub fn save(
    snapshot: LargeSaveSnapshot,
    replacing: &Path,
    available_capacity: impl Fn(&Path) -> Option<u64>,
    progress: impl Fn(f64),
    cancelled: &AtomicBool,
) -> Result<(), LargeFileSaveError> {
    let prepared = LargeFileSaver::prepare(
        &snapshot,
        replacing,
        available_capacity,
        progress,
        cancelled,
    )?;
    // The snapshot holds a mapping of the file being replaced; Windows rejects
    // the swap while it is alive.
    drop(snapshot);
    prepared.install()
}

/// The streaming saver's two-phase API.
pub struct LargeFileSaver;

impl LargeFileSaver {
    /// Validates the save, writes the temporary file next to `target` and
    /// returns it without replacing `target`.
    ///
    /// The caller must keep the original mapping alive while this runs, then
    /// release it before calling [`PreparedSave::install`]: Windows cannot
    /// replace a mapped file. A dropped [`PreparedSave`] removes the temporary
    /// file and leaves the target untouched.
    ///
    /// The checks and the temporary file match [`save`]: the index must be
    /// complete, the mapped source unchanged, and the target volume must have
    /// twice the file's size free. `available_capacity` receives the target
    /// path; its nearest existing ancestor reports the volume for a Save As
    /// target that does not exist yet.
    pub fn prepare(
        snapshot: &LargeSaveSnapshot,
        target: &Path,
        available_capacity: impl Fn(&Path) -> Option<u64>,
        progress: impl Fn(f64),
        cancelled: &AtomicBool,
    ) -> Result<PreparedSave, LargeFileSaveError> {
        ensure_ready(snapshot)?;
        let required = u64::try_from(snapshot.file.size())
            .unwrap_or(u64::MAX)
            .saturating_mul(2);
        if let Some(free) = available_capacity(target)
            && free < required
        {
            return Err(LargeFileSaveError::InsufficientDiskSpace);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }

        let parent = platform::parent_directory(target);
        let name = target.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "the save target has no file name",
            )
        })?;
        let directory = tempfile::Builder::new()
            .prefix(platform::TEMPORARY_PREFIX)
            .tempdir_in(parent)?;
        let temporary = directory.path().join(name);

        let (output, _) = create_and_stream(
            snapshot,
            &temporary,
            COPY_BUFFER_BYTES,
            &progress,
            cancelled,
        )?;
        output.sync_all()?;
        drop(output);
        #[cfg(unix)]
        copy_permissions(target, &temporary)?;

        Ok(PreparedSave {
            target: target.to_path_buf(),
            temporary,
            directory: Some(directory),
        })
    }
}

/// A fully written save that has not replaced the target yet.
///
/// [`PreparedSave::install`] moves the temporary file onto the target. Dropping
/// the handle instead removes the temporary file, so a cancelled or failed
/// save never leaves one behind. The target is untouched until `install`.
#[derive(Debug)]
pub struct PreparedSave {
    target: PathBuf,
    temporary: PathBuf,
    /// Removed on drop; holds the temporary file until `install` renames it out.
    directory: Option<tempfile::TempDir>,
}

impl PreparedSave {
    /// Replaces `target` with the prepared file and syncs its parent.
    ///
    /// An existing target is replaced atomically where the platform allows; a
    /// missing target is created by a plain rename. The target is replaced
    /// before the parent sync, so a failed sync is reported to the error stream
    /// and does not fail the save; the written bytes are already in place.
    pub fn install(self) -> Result<(), LargeFileSaveError> {
        self.install_with_directory_sync(platform::sync_directory)
    }

    /// Replaces `target` and syncs its parent with `sync`.
    ///
    /// This is the testable form of [`PreparedSave::install`]. Only a failure
    /// before the replace is returned as `Err`; a failing sync after the
    /// replace is logged, because the target already holds the saved bytes and
    /// the caller must adopt them.
    #[doc(hidden)]
    pub fn install_with_directory_sync(
        mut self,
        sync: impl FnOnce(&Path) -> io::Result<()>,
    ) -> Result<(), LargeFileSaveError> {
        platform::install_file(&self.temporary, &self.target)?;
        if let Err(error) = sync(platform::parent_directory(&self.target)) {
            eprintln!(
                "tabtool: saved {} but could not sync its directory: {error}",
                self.target.display()
            );
        }
        self.directory.take();
        Ok(())
    }
}

impl Drop for PreparedSave {
    fn drop(&mut self) {
        // The guard removes the private directory and any file still in it.
        // `install` renames the file out first, so on success only the empty
        // directory is removed.
        self.directory.take();
    }
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

fn ensure_ready(snapshot: &LargeSaveSnapshot) -> Result<(), LargeFileSaveError> {
    if snapshot.index.scanned_bytes < snapshot.file.size() {
        return Err(LargeFileSaveError::IndexingNotFinished);
    }
    if snapshot.file.is_stale() != Staleness::Unchanged {
        return Err(LargeFileSaveError::SourceChanged);
    }
    Ok(())
}

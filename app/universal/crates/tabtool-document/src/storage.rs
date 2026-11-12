//! Small-file I/O for document saves.
//!
//! The document layer never opens files directly: every read and write goes
//! through [`Storage`], so tests and other platforms can substitute their own
//! implementation. [`StdStorage`] is the filesystem implementation.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tabtool_large::replace_file;

/// Prefix of the temporary file a [`Storage::write_atomic`] leaves in the
/// target directory while the new bytes are being written.
const TEMPORARY_PREFIX: &str = ".tabtool-save-";

/// Byte access to the file behind a document.
pub trait Storage {
    /// Reads the whole file.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Replaces the file with `data` atomically where the platform allows.
    ///
    /// A reader either sees the old bytes or the new bytes, never a partial
    /// write. An existing file keeps its permissions where the platform
    /// supports that.
    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()>;
}

/// [`Storage`] backed by the standard filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdStorage;

impl Storage for StdStorage {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write_atomic(&self, path: &Path, data: &[u8]) -> io::Result<()> {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        if path.file_name().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the save target has no file name",
            ));
        }

        let mut temporary = tempfile::Builder::new()
            .prefix(TEMPORARY_PREFIX)
            .tempfile_in(parent)?;
        temporary.write_all(data)?;
        temporary.as_file().sync_all()?;
        #[cfg(unix)]
        copy_permissions(path, temporary.path())?;
        // Close the file and stop `TempPath` from deleting the path on drop.
        // Keeping it after the rename would fail on Windows, where keeping
        // clears the temporary attribute by path, because that path is gone.
        let temporary = temporary
            .into_temp_path()
            .keep()
            .map_err(|error| error.error)?;
        if let Err(error) = install(path, &temporary) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }
}

/// Moves `temporary` onto `target`, replacing an existing file atomically.
fn install(target: &Path, temporary: &Path) -> io::Result<()> {
    if target.try_exists()? {
        replace_file(target, temporary)
    } else {
        // `ReplaceFileW` needs an existing target; a new file is a plain rename.
        fs::rename(temporary, target)
    }
}

/// Gives `target` the permissions of `source` when `source` exists.
///
/// The temporary file is created private (`0600`), so a document with other
/// permissions would otherwise narrow to `0600` after a save. A missing source
/// is a new file and keeps the default.
#[cfg(unix)]
fn copy_permissions(source: &Path, target: &Path) -> io::Result<()> {
    match fs::metadata(source) {
        Ok(metadata) => fs::set_permissions(target, metadata.permissions()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

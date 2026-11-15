//! Small-file I/O for document saves.
//!
//! The document layer never opens files directly: every read and write goes
//! through [`Storage`], so tests and other platforms can substitute their own
//! implementation. [`StdStorage`] is the filesystem implementation.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tabtool_large::{replace_file, sync_directory};

/// Prefix of the temporary file a [`Storage::write_atomic`] leaves in the
/// target directory while the new bytes are being written.
const TEMPORARY_PREFIX: &str = ".tabtool-save-";

/// Mode of a new document: owner read/write, group and others read.
#[cfg(unix)]
const NEW_FILE_MODE: u32 = 0o644;

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
        apply_permissions(path, temporary.path())?;
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
        // The bytes are in place; a directory-sync failure is a durability
        // warning like the large saver's, not a reason to report a failed save.
        if let Err(error) = sync_directory(parent) {
            eprintln!(
                "tabtool: saved {} but could not sync its directory: {error}",
                path.display()
            );
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

/// Gives `target` the permissions `source` had, or the default for a new file.
///
/// The temporary file is created private (`0600`), so without this step a save
/// would narrow an existing document's mode or create a new document only its
/// owner can read. A new file gets `0644` explicitly; this does not consult the
/// process umask, which the standard library does not expose, so a restrictive
/// umask is not observed for saves that create a file.
#[cfg(unix)]
fn apply_permissions(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = match fs::metadata(source) {
        Ok(metadata) => metadata.permissions(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::Permissions::from_mode(NEW_FILE_MODE)
        }
        Err(error) => return Err(error),
    };
    fs::set_permissions(target, permissions)
}

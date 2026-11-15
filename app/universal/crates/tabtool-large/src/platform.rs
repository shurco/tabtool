//! Operating-system file operations: mapping, positioned reads, atomic
//! replacement and writes, and free space.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use memmap2::{Mmap, MmapOptions};

/// Maps exactly `size` bytes of `file` read-only.
pub(crate) fn map_read_only(file: &File, size: usize) -> io::Result<Mmap> {
    // SAFETY: the mapping is read-only and `MappedFile` owns the descriptor for
    // at least as long as the mapping: its mapping field is declared before the
    // identity that owns the descriptor, so it is dropped first. A concurrent
    // truncation can make pages unreadable; callers check `is_readable` before
    // trusting the bytes.
    unsafe { MmapOptions::new().len(size).map(file) }
}

/// Reads into `buffer` at `offset` without moving the file cursor.
#[cfg(unix)]
pub(crate) fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buffer, offset)
}

/// Reads into `buffer` at `offset` without moving the file cursor.
#[cfg(windows)]
pub(crate) fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buffer, offset)
}

/// Reads into `buffer` at `offset` without moving the file cursor.
#[cfg(not(any(unix, windows)))]
pub(crate) fn read_at(_file: &File, _buffer: &mut [u8], _offset: u64) -> io::Result<usize> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "positioned reads are not supported",
    ))
}

/// Replaces `target` with `replacement`, atomically where the platform allows.
///
/// Both files must be on the same filesystem. On Windows both must exist; on
/// Unix `replacement` is renamed onto `target`.
#[cfg(unix)]
pub fn replace_file(target: &Path, replacement: &Path) -> io::Result<()> {
    std::fs::rename(replacement, target)
}

/// Replaces `target` with `replacement`, atomically where the platform allows.
///
/// Both files must be on the same filesystem. On Windows both must exist; on
/// Unix `replacement` is renamed onto `target`.
#[cfg(windows)]
pub fn replace_file(target: &Path, replacement: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW};

    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    let replacement: Vec<u16> = replacement
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both names are NUL-terminated wide strings that outlive the call.
    // A null backup name, exclude handle and preserved handle are allowed.
    let replaced = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_IGNORE_MERGE_ERRORS,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Replaces `target` with `replacement`, atomically where the platform allows.
#[cfg(not(any(unix, windows)))]
pub fn replace_file(_target: &Path, _replacement: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic replacement is not supported",
    ))
}

/// Flushes a directory entry change (a rename) to disk.
#[cfg(unix)]
pub fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

/// Directory syncs are a Unix durability step; other platforms have none.
#[cfg(not(unix))]
pub fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Prefix of the temporary file or directory a save leaves next to its target
/// while the new bytes are being written.
pub(crate) const TEMPORARY_PREFIX: &str = ".tabtool-save-";

/// Replaces the file at `path` with `data` atomically where the platform allows.
///
/// A reader either sees the old bytes or the new bytes, never a partial write.
/// The bytes are synced before the swap, and the parent directory after it. An
/// existing file keeps its permissions; a new file gets `new_file_mode` on Unix
/// (other platforms ignore it). The parent directory must already exist.
pub fn write_atomic(path: &Path, data: &[u8], new_file_mode: u32) -> io::Result<()> {
    let parent = parent_directory(path);
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
    apply_permissions(path, temporary.path(), new_file_mode)?;
    #[cfg(not(unix))]
    let _ = new_file_mode;
    // Close the file and stop `TempPath` from deleting the path on drop.
    // Keeping it after the rename would fail on Windows, where keeping
    // clears the temporary attribute by path, because that path is gone.
    let temporary = temporary
        .into_temp_path()
        .keep()
        .map_err(|error| error.error)?;
    if let Err(error) = install_file(&temporary, path) {
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

/// The directory holding `target`, or `.` for a bare file name.
pub(crate) fn parent_directory(target: &Path) -> &Path {
    match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Moves `temporary` onto `target`, replacing an existing file atomically.
pub(crate) fn install_file(temporary: &Path, target: &Path) -> io::Result<()> {
    if target.try_exists()? {
        replace_file(target, temporary)
    } else {
        // `ReplaceFileW` needs an existing target; a new file is a plain rename.
        fs::rename(temporary, target)
    }
}

/// Gives `target` the permissions `source` had, or `new_file_mode` for a new
/// file.
///
/// The temporary file is created private (`0600`), so without this step a save
/// would narrow an existing document's mode or create a new document only its
/// owner can read. A new file gets `new_file_mode` explicitly; this does not
/// consult the process umask, which the standard library does not expose, so a
/// restrictive umask is not observed for saves that create a file.
#[cfg(unix)]
fn apply_permissions(source: &Path, target: &Path, new_file_mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = match fs::metadata(source) {
        Ok(metadata) => metadata.permissions(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::Permissions::from_mode(new_file_mode)
        }
        Err(error) => return Err(error),
    };
    fs::set_permissions(target, permissions)
}

/// Free bytes available to the current user on the volume holding `path`.
pub fn available_space(path: &Path) -> io::Result<u64> {
    fs2::available_space(path)
}

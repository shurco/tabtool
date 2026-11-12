//! Operating-system file operations: mapping, positioned reads, atomic
//! replacement and free space.

use std::fs::File;
use std::io;
use std::path::Path;

use memmap2::{Mmap, MmapOptions};

/// Maps exactly `size` bytes of `file` read-only.
pub(crate) fn map_read_only(file: &File, size: usize) -> io::Result<Mmap> {
    // SAFETY: the mapping is read-only and `MappedFile` keeps the descriptor
    // open for as long as the mapping lives. A concurrent truncation can make
    // pages unreadable; callers check `is_readable` before trusting the bytes.
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

/// Free bytes available to the current user on the volume holding `path`.
pub fn available_space(path: &Path) -> io::Result<u64> {
    fs2::available_space(path)
}

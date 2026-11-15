//! Small-file I/O for document saves.
//!
//! The document layer never opens files directly: every read and write goes
//! through [`Storage`], so tests and other platforms can substitute their own
//! implementation. [`StdStorage`] is the filesystem implementation.

use std::fs;
use std::io;
use std::path::Path;

/// Mode of a new document: owner read/write, group and others read.
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
        tabtool_large::write_atomic(path, data, NEW_FILE_MODE)
    }
}

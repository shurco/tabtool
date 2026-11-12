//! Read-only memory mapping with identity and staleness tracking.

use std::fs::{File, Metadata};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use memmap2::Mmap;
use same_file::Handle;

use crate::platform;

/// Size, modification time and file key of a file, used to notice outside
/// changes.
#[derive(Debug, Eq, PartialEq)]
pub struct FileIdentity {
    size: u64,
    modified_nanos: Option<u128>,
    handle: Handle,
}

impl FileIdentity {
    fn capture(file: File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        let size = metadata.len();
        let modified_nanos = modified_nanos(&metadata);
        let handle = Handle::from_file(file)?;
        Ok(Self {
            size,
            modified_nanos,
            handle,
        })
    }

    /// File size in bytes recorded at open.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Modification time in nanoseconds since the Unix epoch, when available.
    pub fn modified_nanos(&self) -> Option<u128> {
        self.modified_nanos
    }
}

/// Whether the file on disk still matches the mapped one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Staleness {
    /// The same bytes are still on disk.
    Unchanged,
    /// The open file grew; the mapping still covers the recorded size.
    Appended,
    /// The open file shrank or the path now names another file.
    ReplacedOrTruncated,
}

/// A read-only memory mapping of a file.
///
/// The descriptor, mapping and identity never change after [`MappedFile::open`],
/// so the type can be shared between threads. Reads past the mapped size are
/// clamped; a file truncated by another process can still fault on access, so
/// callers check [`MappedFile::is_readable`] first.
#[derive(Debug)]
pub struct MappedFile {
    path: PathBuf,
    size: usize,
    identity: FileIdentity,
    mapping: Option<Mmap>,
}

impl MappedFile {
    /// Opens `path` and maps it read-only. Empty files map nothing and still
    /// answer every range with an empty slice.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let identity = FileIdentity::capture(file)?;
        let size = usize::try_from(identity.size).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "file is larger than the address space",
            )
        })?;
        let mapping = if size > 0 {
            Some(platform::map_read_only(identity.handle.as_file())?)
        } else {
            None
        };
        Ok(Self {
            path: path.to_path_buf(),
            size,
            identity,
            mapping,
        })
    }

    /// Size in bytes recorded at open.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Identity recorded at open: size, modification time and file key.
    pub fn identity(&self) -> &FileIdentity {
        &self.identity
    }

    /// Calls `body` with the bytes of `range`, clamped to the file.
    pub fn with_bytes<R>(&self, range: Range<usize>, body: impl FnOnce(&[u8]) -> R) -> R {
        let lower = range.start.min(self.size);
        let upper = range.end.max(lower).min(self.size);
        match &self.mapping {
            Some(mapping) if upper > lower => body(&mapping[lower..upper]),
            _ => body(&[]),
        }
    }

    /// Reads the bytes of `range`, clamped to the file, into `buffer` and calls
    /// `body` with the bytes read; fewer than asked when the file was truncated
    /// meanwhile. `buffer` grows as needed and can be reused.
    ///
    /// For sequential scans: walking a mapping faults pages in one by one and
    /// keeps them resident, while positioned reads hold only `buffer` and never
    /// fault on a truncated file.
    pub fn read<R>(
        &self,
        range: Range<usize>,
        buffer: &mut Vec<u8>,
        body: impl FnOnce(&[u8]) -> R,
    ) -> R {
        let lower = range.start.min(self.size);
        let upper = range.end.max(lower).min(self.size);
        let wanted = upper - lower;
        if buffer.len() < wanted {
            buffer.resize(wanted, 0);
        }

        let mut done = 0;
        while done < wanted {
            match platform::read_at(
                self.identity.handle.as_file(),
                &mut buffer[done..wanted],
                (lower + done) as u64,
            ) {
                Ok(0) => break,
                Ok(count) => done += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        body(&buffer[..done])
    }

    /// Whether the mapped bytes can still be read: the open file was neither
    /// truncated nor rewritten (it may have grown). Another file now at the
    /// path does not matter; the descriptor keeps the mapped one alive.
    pub fn is_readable(&self) -> bool {
        self.open_staleness() != Staleness::ReplacedOrTruncated
    }

    /// Compares the path and the open descriptor with the identity recorded at
    /// open.
    pub fn is_stale(&self) -> Staleness {
        match Handle::from_path(&self.path) {
            Ok(handle) if handle == self.identity.handle => self.open_staleness(),
            _ => Staleness::ReplacedOrTruncated,
        }
    }

    fn open_staleness(&self) -> Staleness {
        let metadata = match self.identity.handle.as_file().metadata() {
            Ok(metadata) => metadata,
            Err(_) => return Staleness::ReplacedOrTruncated,
        };
        if metadata.len() == self.identity.size
            && modified_nanos(&metadata) == self.identity.modified_nanos
        {
            return Staleness::Unchanged;
        }
        if metadata.len() > self.identity.size {
            Staleness::Appended
        } else {
            Staleness::ReplacedOrTruncated
        }
    }
}

fn modified_nanos(metadata: &Metadata) -> Option<u128> {
    let modified = metadata.modified().ok()?;
    let elapsed = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(elapsed.as_nanos())
}

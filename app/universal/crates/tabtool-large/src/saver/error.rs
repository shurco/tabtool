//! Why a large-file save fails.

use std::io;

use tabtool_core::CsvError;

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

impl Clone for LargeFileSaveError {
    /// Rebuilds the typed value. An I/O error keeps its kind and message but
    /// not its platform error code, because [`io::Error`] is not cloneable.
    fn clone(&self) -> Self {
        match self {
            Self::IndexingNotFinished => Self::IndexingNotFinished,
            Self::SourceChanged => Self::SourceChanged,
            Self::InsufficientDiskSpace => Self::InsufficientDiskSpace,
            Self::Csv(error) => Self::Csv(*error),
            Self::Io(error) => Self::Io(io::Error::new(error.kind(), error.to_string())),
        }
    }
}

pub(super) fn cancelled_error() -> LargeFileSaveError {
    LargeFileSaveError::Io(io::Error::new(io::ErrorKind::Interrupted, "save cancelled"))
}

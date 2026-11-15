//! Ported from `MappedFileTests.swift`, plus the large-file constants.

use std::fs::{self, OpenOptions};
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use tabtool_large::replace_file;
use tabtool_large::{LargeFile, MappedFile, Staleness, available_space};

/// A scratch directory removed when the value goes away.
struct TempDirectory {
    directory: tempfile::TempDir,
}

impl TempDirectory {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temporary directory is created"),
        }
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path().join(name);
        fs::write(&path, bytes).expect("fixture file is written");
        path
    }
}

fn bytes(file: &MappedFile, range: Range<usize>) -> Vec<u8> {
    file.with_bytes(range, <[u8]>::to_vec)
}

fn append(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .expect("fixture file opens for appending");
    io::Write::write_all(&mut file, bytes).expect("bytes are appended");
}

#[cfg(unix)]
fn truncate(path: &Path, size: u64) {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .expect("fixture file opens for truncation");
    file.set_len(size).expect("fixture file is truncated");
}

#[test]
fn maps_content() {
    let directory = TempDirectory::new();
    let file = MappedFile::open(&directory.file("a.csv", b"hello,world\n")).expect("file opens");

    assert_eq!(file.size(), 12);
    assert_eq!(file.identity().size(), 12);
    assert!(file.identity().modified_nanos().is_some());
    assert_eq!(bytes(&file, 0..12), b"hello,world\n");
    assert_eq!(bytes(&file, 6..11), b"world");
}

#[test]
fn empty_file_has_zero_size_and_no_mapping() {
    let directory = TempDirectory::new();
    let file = MappedFile::open(&directory.file("e.csv", b"")).expect("empty file opens");

    assert_eq!(file.size(), 0);
    assert!(bytes(&file, 0..10).is_empty());
    assert!(bytes(&file, 0..0).is_empty());
    assert_eq!(file.is_stale(), Staleness::Unchanged);
    assert!(file.is_readable());
}

#[test]
fn clamps_range() {
    let directory = TempDirectory::new();
    let file = MappedFile::open(&directory.file("a.csv", &[1, 2, 3, 4])).expect("file opens");

    assert_eq!(bytes(&file, 2..100), [3, 4]);
    assert!(bytes(&file, 50..60).is_empty());
}

#[test]
fn read_gives_the_same_bytes_as_the_mapping() {
    let directory = TempDirectory::new();
    let content: Vec<u8> = (0..20_000_u32).map(|index| (index % 251) as u8).collect();
    let file = MappedFile::open(&directory.file("a.csv", &content)).expect("file opens");

    let mut buffer = Vec::new();
    for range in [
        0..20_000,
        5_000..13_001,
        4_096..8_192,
        19_990..20_500,
        30_000..31_000,
        7..7,
    ] {
        let expected = bytes(&file, range.clone());
        assert_eq!(file.read(range, &mut buffer, <[u8]>::to_vec), expected);
    }
}

#[test]
fn read_reuses_the_buffer_and_grows_it_only_as_needed() {
    let directory = TempDirectory::new();
    let content: Vec<u8> = (0..20_000_u32).map(|index| (index % 251) as u8).collect();
    let file = MappedFile::open(&directory.file("a.csv", &content)).expect("file opens");

    let mut buffer = Vec::new();
    let small = file.read(0..10, &mut buffer, <[u8]>::to_vec);
    assert_eq!(small, content[..10]);
    assert_eq!(buffer.len(), 10);
    let large = file.read(0..20_000, &mut buffer, <[u8]>::to_vec);
    assert_eq!(large, content);
    assert_eq!(buffer.len(), 20_000);
    let again = file.read(5..15, &mut buffer, <[u8]>::to_vec);
    assert_eq!(again, content[5..15]);
    assert_eq!(buffer.len(), 20_000, "the buffer is not shrunk");
}

#[test]
fn read_of_an_empty_file_is_empty() {
    let directory = TempDirectory::new();
    let file = MappedFile::open(&directory.file("e.csv", b"")).expect("empty file opens");

    let mut buffer = Vec::new();
    assert!(file.read(0..10, &mut buffer, <[u8]>::to_vec).is_empty());
}

// Windows rejects truncating, replacing or deleting a file while a mapped view
// of it is open (`ERROR_USER_MAPPED_FILE`), so these four states are only
// reachable on Unix.

#[cfg(unix)]
#[test]
fn read_stops_at_a_truncated_end() {
    let directory = TempDirectory::new();
    let path = directory.file("a.csv", &[7; 20_000]);
    let file = MappedFile::open(&path).expect("file opens");
    truncate(&path, 100);

    let mut buffer = Vec::new();
    assert_eq!(file.read(0..20_000, &mut buffer, |bytes| bytes.len()), 100);
}

#[test]
fn detects_unchanged() {
    let directory = TempDirectory::new();
    let file = MappedFile::open(&directory.file("a.csv", &[1, 2, 3])).expect("file opens");

    assert_eq!(file.is_stale(), Staleness::Unchanged);
    assert!(file.is_readable());
}

#[test]
fn detects_append() {
    let directory = TempDirectory::new();
    let path = directory.file("a.csv", &[1, 2, 3]);
    let file = MappedFile::open(&path).expect("file opens");
    append(&path, &[4, 5]);

    assert_eq!(file.is_stale(), Staleness::Appended);
    assert!(file.is_readable());
    assert_eq!(bytes(&file, 0..3), [1, 2, 3], "the mapping keeps its size");
}

#[cfg(unix)]
#[test]
fn detects_truncation() {
    let directory = TempDirectory::new();
    let path = directory.file("a.csv", &[1, 2, 3, 4]);
    let file = MappedFile::open(&path).expect("file opens");
    truncate(&path, 2);

    assert_eq!(file.is_stale(), Staleness::ReplacedOrTruncated);
    assert!(!file.is_readable());
}

#[cfg(unix)]
#[test]
fn detects_replace_and_keeps_reading_the_open_file() {
    let directory = TempDirectory::new();
    let path = directory.file("a.csv", &[1, 2, 3]);
    let file = MappedFile::open(&path).expect("file opens");
    // The replacement has the same bytes and size, so the file key is the only
    // discriminator.
    let other = directory.file("b.csv", &[1, 2, 3]);
    replace_file(&path, &other).expect("replacement succeeds");

    assert_eq!(file.is_stale(), Staleness::ReplacedOrTruncated);
    // The open descriptor still holds the old file.
    assert!(file.is_readable());
    assert_eq!(bytes(&file, 0..3), [1, 2, 3]);
}

#[cfg(unix)]
#[test]
fn a_missing_path_reads_as_replaced_but_stays_readable() {
    let directory = TempDirectory::new();
    let path = directory.file("a.csv", &[1, 2, 3]);
    let file = MappedFile::open(&path).expect("file opens");
    fs::remove_file(&path).expect("fixture file is removed");

    assert_eq!(file.is_stale(), Staleness::ReplacedOrTruncated);
    assert!(file.is_readable(), "the descriptor keeps the file alive");
}

#[test]
fn missing_file_returns_an_error() {
    let directory = TempDirectory::new();
    let path = directory.path().join("missing.csv");

    assert!(MappedFile::open(&path).is_err());
}

#[test]
fn available_space_reports_free_bytes() {
    let directory = TempDirectory::new();

    let space = available_space(directory.path()).expect("free space is readable");
    assert!(space > 0, "a writable temporary directory has free space");
}

#[test]
fn large_file_constants_match_the_agreed_limits() {
    assert_eq!(LargeFile::THRESHOLD_BYTES, 104_857_600);
    assert_eq!(LargeFile::BLOCK_ROWS, 64);
    assert_eq!(LargeFile::SAMPLE_BYTES, 1_048_576);
    assert_eq!(LargeFile::MAX_COLUMNS, 2_048);
    assert_eq!(LargeFile::MAX_COPY_CELLS, 1_000_000);
}

#[test]
fn mapped_file_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<MappedFile>();
}

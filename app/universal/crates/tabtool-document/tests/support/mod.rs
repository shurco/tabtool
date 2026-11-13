//! Shared fixtures for the large-save tests.

#![allow(dead_code)] // Each test crate uses a different subset of these helpers.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, LargeStatus};
use tabtool_large::{LargeCSVSource, LargeFileSaveError};

/// A temporary directory that outlives the mapped file.
pub struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temporary directory"),
        }
    }

    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub fn path(&self) -> PathBuf {
        self.directory.path().join("big.csv")
    }

    pub fn copy_path(&self) -> PathBuf {
        self.directory.path().join("copy.csv")
    }

    /// Rows like `r5,v5,"q 5"`; the second field repeats every seven rows.
    pub fn csv(&self, rows: usize) -> PathBuf {
        let path = self.path();
        let text = (0..rows)
            .map(|row| format!("r{row},v{},\"q {row}\"", row % 7))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, text).expect("fixture file is written");
        path
    }

    /// A large fixture written straight to disk, so multi-megabyte files do
    /// not need to be formatted into one `String` first.
    pub fn large_csv(&self, rows: usize) -> PathBuf {
        let path = self.path();
        let file = std::fs::File::create(&path).expect("fixture file is created");
        let mut writer = std::io::BufWriter::new(file);
        for row in 0..rows {
            writeln!(writer, "r{row},v{},\"q {row}\"", row % 7).expect("fixture row is written");
        }
        writer.flush().expect("fixture is flushed");
        path
    }
}

pub fn open_ready(path: &Path) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(path, None, false, 0)
        .expect("open the large fixture");
    finish_indexing(&mut document);
    document
}

pub fn finish_indexing(document: &mut Document) {
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
}

/// A ready large document with no header row, like the Swift `open` helper.
pub fn open_without_header(path: &Path) -> Document {
    let mut document = open_ready(path);
    if document.configuration().first_row_as_header {
        document.set_large_header_enabled(false);
    }
    document
}

/// A document whose indexer was cancelled after its first step, so the index
/// gate can be observed deterministically.
pub fn open_indexing(path: &Path) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(path, None, false, 0)
        .expect("open the large fixture");
    let state = document.large_state().expect("large state");
    assert!(
        !state.indexer.is_finished(),
        "the indexer finished before it could be paused"
    );
    state.indexer.cancel();
    state.indexer.wait();
    document.poll_large_indexing();
    document
}

pub fn source(document: &Document) -> Arc<LargeCSVSource> {
    Arc::clone(&document.large_state().expect("large state").source)
}

pub fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("fixture is readable")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(unix)]
pub fn inode(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("fixture metadata").ino()
}

#[cfg(not(unix))]
pub fn inode(_path: &Path) -> u64 {
    0
}

pub fn start_error(document: &mut Document, destination: Option<&Path>) -> LargeFileSaveError {
    match document.start_large_save(destination) {
        Ok(_) => panic!("the large save was expected to be refused"),
        Err(error) => error,
    }
}

/// Waits until the worker reports a perfect fraction; the document keeps the
/// job until `wait_for_large_save`, so the status stays `Saving`.
pub fn wait_until_complete(document: &Document) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match document.large_save_progress() {
            Some(progress) if progress >= 1.0 => return,
            Some(_) => {}
            None => panic!("the save stopped reporting progress"),
        }
        assert!(Instant::now() < deadline, "the large save timed out");
        std::thread::yield_now();
    }
}

pub fn saving(document: &Document) -> bool {
    matches!(document.large_status(), LargeStatus::Saving(_))
}

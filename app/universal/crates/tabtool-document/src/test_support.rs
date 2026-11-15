//! Test doubles and document fixtures shared by the document, UI and desktop
//! test suites. Built only with the `test-support` feature.

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tabtool_core::CsvConfiguration;

use crate::{Document, Storage};

/// In-memory [`Storage`] that counts reads and writes and can fail either.
pub struct MemoryStorage {
    bytes: Mutex<Vec<u8>>,
    reads: AtomicUsize,
    writes: AtomicUsize,
    fail_reads: AtomicBool,
    fail_writes: AtomicBool,
}

impl MemoryStorage {
    /// A storage holding `bytes`.
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: Mutex::new(bytes.to_vec()),
            reads: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            fail_reads: AtomicBool::new(false),
            fail_writes: AtomicBool::new(false),
        }
    }

    /// The bytes most recently stored.
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.lock().expect("unpoisoned storage").clone()
    }

    /// Replaces the stored bytes, as an outside edit would.
    pub fn set_bytes(&self, bytes: &[u8]) {
        *self.bytes.lock().expect("unpoisoned storage") = bytes.to_vec();
    }

    /// How many reads reached the storage.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }

    /// How many writes reached the storage.
    pub fn writes(&self) -> usize {
        self.writes.load(Ordering::SeqCst)
    }

    /// Makes every read fail, or succeed again.
    pub fn fail_reads(&self, fail: bool) {
        self.fail_reads.store(fail, Ordering::SeqCst);
    }

    /// Makes every write fail, or succeed again.
    pub fn fail_writes(&self, fail: bool) {
        self.fail_writes.store(fail, Ordering::SeqCst);
    }
}

impl Storage for MemoryStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fail_reads.load(Ordering::SeqCst) {
            return Err(io::Error::other("the file is unavailable"));
        }
        Ok(self.bytes())
    }

    fn write_atomic(&self, _path: &Path, data: &[u8]) -> io::Result<()> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        if self.fail_writes.load(Ordering::SeqCst) {
            return Err(io::Error::other("the disk is full"));
        }
        self.set_bytes(data);
        Ok(())
    }
}

/// A ready document loaded from `/virtual/document.csv`.
pub fn loaded(storage: &MemoryStorage, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_path(
            Path::new("/virtual/document.csv"),
            storage,
            Some(configuration),
            false,
        )
        .expect("load the fixture");
    document
}

/// A ready untitled document.
pub fn untitled(configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .new_document(configuration)
        .expect("create a new document");
    document
}

/// Opens `path` through the large-file path and waits until its index is ready.
pub fn large_ready(path: &Path) -> Document {
    let mut document = Document::new(CsvConfiguration::default());
    document
        .open_large(path, None, false, 0)
        .expect("open the large fixture");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
    document
}

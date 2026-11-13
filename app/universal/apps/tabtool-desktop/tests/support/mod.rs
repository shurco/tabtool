//! Shared test doubles for the desktop shell integration tests.

#![allow(dead_code)]

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, Storage};

/// In-memory [`Storage`] that counts writes and can fail them.
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

    /// The bytes most recently read or written.
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.lock().expect("unpoisoned storage").clone()
    }

    /// How many writes reached the storage.
    pub fn writes(&self) -> usize {
        self.writes.load(Ordering::SeqCst)
    }

    /// Makes every write fail.
    pub fn fail_writes(&self) {
        self.fail_writes.store(true, Ordering::SeqCst);
    }

    /// Makes every read fail.
    pub fn fail_reads(&self) {
        self.fail_reads.store(true, Ordering::SeqCst);
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
        *self.bytes.lock().expect("unpoisoned storage") = data.to_vec();
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

/// A ready in-memory document with `csv` content.
pub fn loaded_csv(_storage: &MemoryStorage, csv: &[u8]) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            csv,
            Some(Path::new("/virtual/document.csv")),
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

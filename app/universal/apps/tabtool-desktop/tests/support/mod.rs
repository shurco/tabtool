//! Shared test doubles for the desktop shell integration tests.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, Storage};
use tabtool_platform::FileDialogs;

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

/// A ready untitled document.
pub fn untitled(configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .new_document(configuration)
        .expect("create a new document");
    document
}

/// A scripted file dialog source: one open result and one save path per call.
pub struct FakeDialogs {
    opens: Mutex<VecDeque<Vec<PathBuf>>>,
    results: Mutex<VecDeque<Option<PathBuf>>>,
    saves: Mutex<Vec<(String, String)>>,
    open_calls: AtomicUsize,
}

impl FakeDialogs {
    /// A panel that returns `results` in order.
    pub fn scripted(results: impl IntoIterator<Item = Option<PathBuf>>) -> Arc<Self> {
        Arc::new(Self {
            opens: Mutex::new(VecDeque::new()),
            results: Mutex::new(results.into_iter().collect()),
            saves: Mutex::new(Vec::new()),
            open_calls: AtomicUsize::new(0),
        })
    }

    /// A panel that only cancels.
    pub fn cancelled() -> Arc<Self> {
        Self::scripted([None])
    }

    /// An open panel that returns `results` in order.
    pub fn scripted_opens(results: impl IntoIterator<Item = Vec<PathBuf>>) -> Arc<Self> {
        let dialogs = Self::cancelled();
        *dialogs.opens.lock().expect("unpoisoned dialogs") = results.into_iter().collect();
        dialogs
    }

    /// The suggested names and extensions the app asked for.
    pub fn saves(&self) -> Vec<(String, String)> {
        self.saves.lock().expect("unpoisoned dialogs").clone()
    }

    /// How many times the open panel was shown.
    pub fn open_calls(&self) -> usize {
        self.open_calls.load(Ordering::SeqCst)
    }
}

impl FileDialogs for FakeDialogs {
    fn open_files(&self) -> Vec<PathBuf> {
        self.open_calls.fetch_add(1, Ordering::SeqCst);
        self.opens
            .lock()
            .expect("unpoisoned dialogs")
            .pop_front()
            .unwrap_or_default()
    }

    fn save_file(&self, suggested_name: &str, extension: &str) -> Option<PathBuf> {
        self.saves
            .lock()
            .expect("unpoisoned dialogs")
            .push((suggested_name.to_owned(), extension.to_owned()));
        self.results
            .lock()
            .expect("unpoisoned dialogs")
            .pop_front()
            .flatten()
    }

    fn choose_folder(&self) -> Option<PathBuf> {
        None
    }
}

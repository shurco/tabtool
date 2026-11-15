//! Shared test doubles for the desktop shell integration tests.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tabtool_platform::FileDialogs;

#[allow(unused_imports)] // Each test crate re-exports a different subset.
pub use tabtool_document::test_support::{MemoryStorage, large_ready, loaded, untitled};

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

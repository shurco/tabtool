//! Scans a mapped file on a background thread and publishes the growing index.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tabtool_core::CsvConfiguration;

use crate::index::RowIndex;
use crate::mapped::MappedFile;
use crate::scanner::RowScanner;

/// A progress report from [`LargeFileIndexer`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexProgress {
    /// Share of the file scanned, in `0.0..=1.0`; `1.0` for an empty file.
    pub fraction: f64,
    /// Rows that have started in the scanned bytes.
    pub row_count: usize,
    /// The most fields seen in one row so far.
    pub max_field_count: usize,
    /// Whether the whole file has been scanned.
    pub is_finished: bool,
}

/// Scans a mapped file on a background thread and publishes the growing
/// [`RowIndex`].
///
/// The file is read in steps of `step_bytes` into one reused buffer. After
/// every step the index so far is available through [`LargeFileIndexer::snapshot`],
/// so a reader can show the first rows while the scan continues.
pub struct LargeFileIndexer {
    file: Arc<MappedFile>,
    configuration: CsvConfiguration,
    step_bytes: usize,
    progress_interval: Duration,
    published: Mutex<RowIndex>,
    finished: AtomicBool,
    cancelled: AtomicBool,
    started: AtomicBool,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl LargeFileIndexer {
    /// Prepares a scan of `file` with the row rules of `configuration`.
    ///
    /// A zero `step_bytes` is treated as one byte so the scan always advances.
    pub fn new(
        file: Arc<MappedFile>,
        configuration: CsvConfiguration,
        step_bytes: usize,
        progress_interval: Duration,
    ) -> Self {
        Self {
            file,
            configuration,
            step_bytes: step_bytes.max(1),
            progress_interval,
            published: Mutex::new(RowIndex::new()),
            finished: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            started: AtomicBool::new(false),
            worker: Mutex::new(None),
        }
    }

    /// Starts scanning once. `on_progress` runs on the scanning thread: right
    /// after the first step, then at most every `progress_interval`, and at the
    /// end. Later calls do nothing.
    pub fn start(self: &Arc<Self>, on_progress: impl Fn(IndexProgress) + Send + 'static) {
        // Hold the worker slot across the spawn so `wait` cannot miss the
        // handle of a scan that has already been requested.
        let mut worker = lock(&self.worker);
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let indexer = Arc::clone(self);
        let handle = thread::spawn(move || indexer.run(on_progress));
        *worker = Some(handle);
    }

    /// The index of the bytes scanned so far.
    pub fn snapshot(&self) -> RowIndex {
        lock(&self.published).clone()
    }

    /// Whether the whole file has been scanned.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    /// Asks the scan to stop after the current step.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Blocks until the scan has finished or stopped. A scan never started, or
    /// one already joined, returns immediately.
    pub fn wait(&self) {
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }

    fn run(&self, on_progress: impl Fn(IndexProgress)) {
        let size = self.file.size();
        let mut scanner = RowScanner::new(&self.configuration);
        // `index` keeps the counters across steps and holds only the
        // checkpoints of the current step; those move into `published`.
        let mut index = RowIndex::new();
        let mut buffer = Vec::new();
        let mut offset = 0usize;
        let mut last_report: Option<Instant> = None;
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            if !self.file.is_readable() {
                return;
            }
            let end = offset.saturating_add(self.step_bytes).min(size);
            let is_last = end == size;
            let wanted = end - offset;
            let mut complete = true;
            self.file.read(offset..end, &mut buffer, |bytes| {
                if bytes.len() < wanted {
                    complete = false;
                    return;
                }
                scanner.scan(bytes, offset, &mut index, is_last);
            });
            // The file was truncated by another process mid-step.
            if !complete {
                return;
            }
            offset = end;

            let step = RowIndex {
                checkpoints: std::mem::take(&mut index.checkpoints),
                row_count: index.row_count,
                max_field_count: index.max_field_count,
                scanned_bytes: index.scanned_bytes,
                ends_inside_quote: index.ends_inside_quote,
            };
            {
                let mut published = lock(&self.published);
                published.extend(step);
                self.finished.store(is_last, Ordering::Release);
            }

            let now = Instant::now();
            let due =
                last_report.is_none_or(|last| now.duration_since(last) >= self.progress_interval);
            if is_last || due {
                last_report = Some(now);
                on_progress(IndexProgress {
                    fraction: if size == 0 {
                        1.0
                    } else {
                        offset as f64 / size as f64
                    },
                    row_count: index.row_count,
                    max_field_count: index.max_field_count,
                    is_finished: is_last,
                });
            }
            if offset >= size {
                break;
            }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

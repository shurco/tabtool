//! Scans a mapped file on a background thread and publishes the growing index.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
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

/// The cheap counters of an index, without its block checkpoints.
///
/// The grid reads these once per cell; copying the whole [`RowIndex`] there
/// would clone every checkpoint of a growing scan.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IndexCounters {
    /// Rows that have started in the scanned bytes.
    pub row_count: usize,
    /// The most fields seen in one row so far.
    pub max_field_count: usize,
    /// Bytes consumed so far.
    pub scanned_bytes: usize,
    /// Whether the whole file has been scanned.
    pub is_finished: bool,
}

/// Scans a mapped file on a background thread and publishes the growing
/// [`RowIndex`].
///
/// The file is read in steps of `step_bytes` into one reused buffer. After
/// every step the index so far is available through [`LargeFileIndexer::snapshot`],
/// so a reader can show the first rows while the scan continues.
///
/// A panic inside the scan or its progress callback is caught on the worker:
/// it leaves [`LargeFileIndexer::is_finished`] false and makes every
/// [`LargeFileIndexer::wait`] caller panic with the original message.
pub struct LargeFileIndexer {
    file: Arc<MappedFile>,
    configuration: CsvConfiguration,
    step_bytes: usize,
    progress_interval: Duration,
    published: Mutex<RowIndex>,
    finished: AtomicBool,
    cancelled: AtomicBool,
    started: AtomicBool,
    completion: Mutex<Completion>,
    completion_signal: Condvar,
    worker: Mutex<Option<JoinHandle<()>>>,
}

/// How the background worker ended, published through `completion_signal`.
#[derive(Default)]
struct Completion {
    done: bool,
    failure: Option<String>,
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
            completion: Mutex::new(Completion::default()),
            completion_signal: Condvar::new(),
            worker: Mutex::new(None),
        }
    }

    /// Rebuilds an indexer over `file` with an index that was already scanned.
    ///
    /// The result is immediately finished and has no worker: [`start`] does
    /// nothing and [`wait`] returns at once. The document uses this to restore
    /// a large source after a failed install without rescanning the file.
    ///
    /// [`start`]: LargeFileIndexer::start
    /// [`wait`]: LargeFileIndexer::wait
    pub fn seeded(
        file: Arc<MappedFile>,
        configuration: CsvConfiguration,
        index: RowIndex,
    ) -> Arc<Self> {
        Arc::new(Self {
            file,
            configuration,
            step_bytes: 1,
            progress_interval: Duration::ZERO,
            published: Mutex::new(index),
            finished: AtomicBool::new(true),
            cancelled: AtomicBool::new(false),
            // Mark the scan as started so `start` cannot launch a worker, and
            // the completion as done so `wait` does not block on a missing one.
            started: AtomicBool::new(true),
            completion: Mutex::new(Completion {
                done: true,
                failure: None,
            }),
            completion_signal: Condvar::new(),
            worker: Mutex::new(None),
        })
    }

    /// Starts scanning once. `on_progress` runs on the scanning thread: right
    /// after the first step, then at most every `progress_interval`, and at the
    /// end. Later calls do nothing. Do not call [`LargeFileIndexer::wait`] from
    /// `on_progress`; it runs on the thread that `wait` joins.
    pub fn start(self: &Arc<Self>, on_progress: impl Fn(IndexProgress) + Send + 'static) {
        // Hold the worker slot across the spawn so `wait` cannot miss the
        // handle of a scan that has already been requested.
        let mut worker = lock(&self.worker);
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let indexer = Arc::clone(self);
        let spawned = thread::Builder::new().spawn(move || {
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| indexer.run(on_progress)));
            let mut completion = lock(&indexer.completion);
            if let Err(payload) = outcome {
                indexer.finished.store(false, Ordering::Release);
                completion.failure = Some(panic_message(payload));
            }
            completion.done = true;
            indexer.completion_signal.notify_all();
        });
        match spawned {
            Ok(handle) => *worker = Some(handle),
            Err(error) => {
                let mut completion = lock(&self.completion);
                completion.failure = Some(format!("indexer worker could not start: {error}"));
                completion.done = true;
                self.completion_signal.notify_all();
            }
        }
    }

    /// The index of the bytes scanned so far.
    pub fn snapshot(&self) -> RowIndex {
        lock(&self.published).clone()
    }

    /// The counters of the index of the bytes scanned so far, without copying
    /// its checkpoints.
    pub fn counters(&self) -> IndexCounters {
        let published = lock(&self.published);
        IndexCounters {
            row_count: published.row_count,
            max_field_count: published.max_field_count,
            scanned_bytes: published.scanned_bytes,
            is_finished: self.finished.load(Ordering::Acquire),
        }
    }

    /// Whether the whole file has been scanned.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    /// Asks the scan to stop after the current step.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Blocks until the scan has finished or stopped, or its worker panicked.
    /// Every caller blocks until then; a scan never started returns
    /// immediately.
    ///
    /// # Panics
    ///
    /// Panics with the worker's message when the scan or its progress callback
    /// panicked. Every later call panics the same way.
    pub fn wait(&self) {
        if !self.started.load(Ordering::Acquire) {
            return;
        }
        let worker = lock(&self.worker).take();
        if let Some(worker) = worker
            && let Err(payload) = worker.join()
        {
            // `start` catches panics inside the worker, so this only covers a
            // panic outside that guard; keep it visible as a failure.
            let mut completion = lock(&self.completion);
            completion
                .failure
                .get_or_insert_with(|| panic_message(payload));
            completion.done = true;
            self.completion_signal.notify_all();
        }
        let mut completion = lock(&self.completion);
        completion = self
            .completion_signal
            .wait_while(completion, |state| !state.done)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let failure = completion.failure.clone();
        drop(completion);
        if let Some(message) = failure {
            panic!("{message}");
        }
    }

    fn run(&self, on_progress: impl Fn(IndexProgress)) {
        let size = self.file.size();
        let mut scanner = RowScanner::new(&self.configuration);
        // `index` keeps the counters across steps and holds only the
        // checkpoints of the current step; those are appended to `published`
        // and then cleared, keeping the capacity for the next step.
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
            // Only a truncation between the readability check and this read
            // reaches the short-read guard; the window is too narrow to hit
            // deterministically in a test, and `MappedFile` tests already cover
            // short reads from the positioned read itself.
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

            {
                let mut published = lock(&self.published);
                published.checkpoints.extend_from_slice(&index.checkpoints);
                published.row_count = index.row_count;
                published.max_field_count = index.max_field_count;
                published.scanned_bytes = index.scanned_bytes;
                published.ends_inside_quote = index.ends_inside_quote;
                self.finished.store(is_last, Ordering::Release);
            }
            index.checkpoints.clear();

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

/// The message of a panic payload, for propagation through [`LargeFileIndexer::wait`].
fn panic_message(payload: Box<dyn Any + Send + 'static>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "indexer worker panicked".to_string()
    }
}

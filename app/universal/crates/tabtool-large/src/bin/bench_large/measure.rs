//! Timing, footprint sampling, deterministic randomness and formatting for the
//! large-file benchmark.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// The process's resident memory: RSS on Unix, the working set on Windows.
pub fn footprint() -> Option<u64> {
    memory_stats::memory_stats().map(|stats| stats.physical_mem as u64)
}

/// Runs `body` while a background thread samples the footprint every 5 ms.
///
/// Returns the body's value and the peak footprint, or `None` when the
/// platform has no memory query.
pub fn peak_footprint<T>(body: impl FnOnce() -> T) -> (T, Option<u64>) {
    let Some(initial) = footprint() else {
        return (body(), None);
    };
    let peak = AtomicU64::new(initial);
    let running = AtomicBool::new(true);
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            while running.load(Ordering::Relaxed) {
                if let Some(now) = footprint() {
                    peak.fetch_max(now, Ordering::Relaxed);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let _stop = Stop(&running);
        body()
    });
    let current = footprint().unwrap_or(0);
    (result, Some(peak.load(Ordering::Relaxed).max(current)))
}

/// Stops the sampler even when `body` panics.
struct Stop<'a>(&'a AtomicBool);

impl Drop for Stop<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

pub fn seconds(duration: Duration) -> String {
    format!("{:.2} s", duration.as_secs_f64())
}

pub fn milliseconds(duration: Duration) -> String {
    format!("{:.3} ms", duration.as_secs_f64() * 1000.0)
}

pub fn gib(bytes: u64) -> String {
    format!("{:.2} GiB", bytes as f64 / 1_073_741_824.0)
}

pub fn mib(bytes: Option<u64>) -> String {
    match bytes {
        Some(bytes) => format!("{} MiB", bytes / (1 << 20)),
        None => "n/a".to_owned(),
    }
}

/// Rows, in millions when large, as the Apple benchmark prints them.
pub fn millions(rows: u64) -> String {
    if rows >= 1_000_000 {
        format!("{:.1} million", rows as f64 / 1_000_000.0)
    } else {
        rows.to_string()
    }
}

/// A compact row count for the scroll label: `100k`, not `100,000`.
pub fn compact_rows(rows: usize) -> String {
    if rows >= 1_000 && rows.is_multiple_of(1_000) {
        format!("{}k", rows / 1_000)
    } else {
        rows.to_string()
    }
}

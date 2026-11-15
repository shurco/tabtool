//! The scan loop the indexer runs on its worker thread.

use std::sync::atomic::Ordering;
use std::time::Instant;

use super::{IndexProgress, LargeFileIndexer};
use crate::index::RowIndex;
use crate::lock;
use crate::scanner::RowScanner;

impl LargeFileIndexer {
    pub(super) fn run(&self, on_progress: impl Fn(IndexProgress)) {
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

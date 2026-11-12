//! Ported from `LargeFileIndexerTests.swift`, plus the background-scan
//! boundaries: multi-step files, repeated start, cancel and truncation.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use support::{LargeCsvFixture, large_file_indexer, scan_index};
use tabtool_core::CsvConfiguration;
use tabtool_large::{IndexProgress, LargeFileIndexer, MappedFile};

const STEP_BYTES: usize = 4096;
const MIB: usize = 1 << 20;

fn indexer(fixture: &LargeCsvFixture) -> Arc<LargeFileIndexer> {
    large_file_indexer(fixture, STEP_BYTES, Duration::ZERO)
}

#[test]
fn final_count_matches_generator() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    indexer.start(|_| {});
    indexer.wait();

    assert!(indexer.is_finished());
    let snapshot = indexer.snapshot();
    assert_eq!(snapshot.row_count, fixture.rows);
    assert_eq!(snapshot.max_field_count, 5);
    assert_eq!(snapshot.scanned_bytes, fixture.file.size());
}

#[test]
fn matches_direct_scan() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    indexer.start(|_| {});
    indexer.wait();

    let configuration = CsvConfiguration::default();
    let size = fixture.file.size();
    let direct = fixture
        .file
        .with_bytes(0..size, |bytes| scan_index(bytes, &configuration));
    assert_eq!(indexer.snapshot(), direct);
}

#[test]
fn partial_snapshots_are_monotone_prefixes() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let collected = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&collected);
    let scan = Arc::clone(&indexer);
    indexer.start(move |_| {
        sink.lock()
            .expect("snapshot list lock")
            .push(scan.snapshot());
    });
    indexer.wait();

    let snapshots = collected.lock().expect("snapshot list lock");
    assert!(snapshots.len() > 10);
    let final_snapshot = indexer.snapshot();
    for pair in snapshots.windows(2) {
        assert!(pair[0].row_count <= pair[1].row_count);
        assert!(pair[0].scanned_bytes <= pair[1].scanned_bytes);
        assert!(pair[1].checkpoints.starts_with(&pair[0].checkpoints));
    }
    for snapshot in snapshots.iter() {
        assert!(
            final_snapshot
                .checkpoints
                .starts_with(&snapshot.checkpoints)
        );
    }
}

#[test]
fn first_snapshot_is_available_before_the_end() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let first = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&first);
    let scan = Arc::clone(&indexer);
    indexer.start(move |progress| {
        let mut first = sink.lock().expect("first snapshot lock");
        if first.is_none() {
            *first = Some((scan.snapshot().row_count, progress.is_finished));
        }
    });
    indexer.wait();

    let seen = first.lock().expect("first snapshot lock");
    let (rows, finished) = seen.expect("progress reported");
    assert!(rows > 0);
    assert!(!finished);
}

#[test]
fn cancel_stops_early() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let scan = Arc::clone(&indexer);
    indexer.start(move |_| scan.cancel());
    indexer.wait();

    assert!(!indexer.is_finished());
    assert!(indexer.snapshot().scanned_bytes < fixture.file.size());
}

#[test]
fn empty_file_is_one_row_with_full_progress() {
    let directory = tempfile::tempdir().expect("fixture directory is created");
    let path = directory.path().join("empty.csv");
    std::fs::write(&path, b"").expect("empty file is written");
    let file = Arc::new(MappedFile::open(&path).expect("empty file opens"));
    let indexer = Arc::new(LargeFileIndexer::new(
        file,
        CsvConfiguration::default(),
        8 * MIB,
        Duration::from_millis(100),
    ));
    let last: Arc<Mutex<Option<IndexProgress>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&last);
    indexer.start(move |progress| {
        *sink.lock().expect("progress lock") = Some(progress);
    });
    indexer.wait();

    assert!(indexer.is_finished());
    assert_eq!(indexer.snapshot().row_count, 1);
    let progress = last
        .lock()
        .expect("progress lock")
        .expect("progress reported");
    assert!(progress.is_finished);
    assert_eq!(progress.fraction, 1.0);
    assert_eq!(progress.row_count, 1);
}

#[test]
fn ends_inside_quote_flag() {
    let directory = tempfile::tempdir().expect("fixture directory is created");
    let path = directory.path().join("quote.csv");
    std::fs::write(&path, b"a,b\n\"open,1\n2,3\n").expect("fixture file is written");
    let file = Arc::new(MappedFile::open(&path).expect("fixture file opens"));
    let indexer = Arc::new(LargeFileIndexer::new(
        file,
        CsvConfiguration::default(),
        STEP_BYTES,
        Duration::ZERO,
    ));
    indexer.start(|_| {});
    indexer.wait();

    assert!(indexer.snapshot().ends_inside_quote);
}

#[test]
fn progress_fraction_reaches_one() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let last: Arc<Mutex<Option<IndexProgress>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&last);
    indexer.start(move |progress| {
        *sink.lock().expect("progress lock") = Some(progress);
    });
    indexer.wait();

    let progress = last
        .lock()
        .expect("progress lock")
        .expect("progress reported");
    assert!(progress.is_finished);
    assert_eq!(progress.fraction, 1.0);
    assert_eq!(progress.row_count, fixture.rows);
}

#[test]
fn file_just_over_one_step_is_scanned_in_two_steps() {
    let fixture = LargeCsvFixture::with_target(8 * MIB + 1);
    assert!(fixture.file.size() > 8 * MIB);
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&fixture.file),
        CsvConfiguration::default(),
        8 * MIB,
        Duration::from_secs(3600),
    ));
    let reports = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&reports);
    indexer.start(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    });
    indexer.wait();

    // The first step always reports; the throttled second report is the last.
    assert_eq!(reports.load(Ordering::Relaxed), 2);
    assert!(indexer.is_finished());
    let configuration = CsvConfiguration::default();
    let size = fixture.file.size();
    let direct = fixture
        .file
        .with_bytes(0..size, |bytes| scan_index(bytes, &configuration));
    assert_eq!(indexer.snapshot(), direct);
}

#[test]
fn wait_after_cancel_returns_and_is_repeatable() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let scan = Arc::clone(&indexer);
    indexer.start(move |_| scan.cancel());
    indexer.wait();
    indexer.wait();

    assert!(!indexer.is_finished());
}

#[test]
fn start_twice_keeps_the_first_scan() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    let first_sink = Arc::clone(&first);
    indexer.start(move |_| {
        first_sink.fetch_add(1, Ordering::Relaxed);
    });
    let second_sink = Arc::clone(&second);
    indexer.start(move |_| {
        second_sink.fetch_add(1, Ordering::Relaxed);
    });
    indexer.wait();

    assert!(first.load(Ordering::Relaxed) > 0);
    assert_eq!(second.load(Ordering::Relaxed), 0);
    assert!(indexer.is_finished());
    assert_eq!(indexer.snapshot().row_count, fixture.rows);
}

#[cfg(unix)]
#[test]
fn truncation_during_the_scan_stops_without_panic() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    let path = fixture.path();
    let truncated = Arc::new(AtomicUsize::new(0));
    let flag = Arc::clone(&truncated);
    indexer.start(move |_| {
        if flag.fetch_add(1, Ordering::Relaxed) == 0 {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("fixture file opens for truncation");
            file.set_len(0).expect("fixture file is truncated");
        }
    });
    indexer.wait();

    assert!(!indexer.is_finished());
    assert!(indexer.snapshot().scanned_bytes < fixture.file.size());
}

#[test]
fn cancel_after_completion_is_a_noop() {
    let fixture = LargeCsvFixture::new();
    let indexer = indexer(&fixture);
    indexer.start(|_| {});
    indexer.wait();
    let before = indexer.snapshot();
    indexer.cancel();
    indexer.wait();

    assert!(indexer.is_finished());
    assert_eq!(indexer.snapshot(), before);
}

//! Benchmarks large-file mode on a generated file.
//!
//! Run it through `xtask bench-large [GiB]` (default 5), which builds this
//! binary in release and enables the `bench` feature. The metrics match
//! `scripts/bench-large` and the Performance section of `README.md`.

#![deny(unsafe_code)]

// The binary is one file with its helpers beside it; explicit paths keep the
// helpers from being discovered as separate binaries.
#[path = "bench_large/cli.rs"]
mod cli;
#[path = "bench_large/measure.rs"]
mod measure;
#[path = "bench_large/verify.rs"]
mod verify;

use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use tabtool_large::{
    CellKey, CellOverlay, LargeCSVSource, LargeFile, LargeFileIndexer, MappedFile, sniff, write,
};
use tabtool_testdata::{LargeOptions, generate_large_csv};

use cli::{ensure_free_space, requested_gib, target_bytes};
use measure::{
    SplitMix64, compact_rows, footprint, gib, mib, millions, milliseconds, peak_footprint, seconds,
};

const COLUMNS: usize = 8;
const MULTILINE_EVERY: usize = 1_000;
const GENERATOR_SEED: u64 = 42;
const RANDOM_SEED: u64 = 7;
const INDEX_STEP_BYTES: usize = 8 << 20;
const INDEX_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const SAVE_BUFFER_BYTES: usize = 4 << 20;
const RANDOM_READS: usize = 1_000;
const SAMPLE_COLUMN: usize = 3;
const FOOTPRINT_SAMPLE_COLUMN: usize = 1;
const SCROLL_ROWS: usize = 100_000;
const SAMPLE_READS: usize = 10_000;
const MAX_EDITS: usize = 1_000;
const EDIT_COUNTS: [usize; 3] = [0, 1, MAX_EDITS];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("bench-large: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let gigabytes = requested_gib(std::env::args().skip(1))?;
    let target = target_bytes(gigabytes);
    let directory = tempfile::Builder::new()
        .prefix("tabtool-bench-large-")
        .tempdir()
        .map_err(|error| format!("cannot create the temporary directory: {error}"))?;
    let path = directory.path().join("bench.csv");
    ensure_free_space(directory.path(), target)?;

    eprintln!("bench-large: generating {} ...", gib(target));
    let generated_at = Instant::now();
    let fixture = generate_large_csv(&LargeOptions {
        path: path.clone(),
        bytes: target,
        columns: COLUMNS,
        multiline_every: Some(MULTILINE_EVERY),
        crlf: false,
        bom: false,
        quoted: true,
        seed: GENERATOR_SEED,
    })
    .map_err(|error| format!("cannot generate {}: {error}", path.display()))?;
    let generation = generated_at.elapsed();

    let mut report = Report::default();
    report.add(
        "File",
        format!(
            "{}, {} rows x {COLUMNS} columns (generated in {})",
            gib(fixture.bytes),
            millions(fixture.rows),
            seconds(generation)
        ),
    );
    report.add("Footprint before open", mib(footprint()));

    let opened = Instant::now();
    let file = Arc::new(
        MappedFile::open(&path)
            .map_err(|error| format!("cannot open {}: {error}", path.display()))?,
    );
    let configuration =
        sniff(&file, false).map_err(|error| format!("cannot detect the format: {error:?}"))?;
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&file),
        configuration.clone(),
        INDEX_STEP_BYTES,
        INDEX_PROGRESS_INTERVAL,
    ));
    indexer.start(|_| {});
    let source = LargeCSVSource::new(
        Arc::clone(&file),
        configuration.clone(),
        Arc::clone(&indexer),
        CellOverlay::new(),
        Vec::new(),
    );
    wait_for_first_block(&source);
    for row in 0..LargeFile::BLOCK_ROWS {
        let _ = source.cell(row, COLUMNS - 1);
    }
    report.add("Open to the first 64 rows", milliseconds(opened.elapsed()));

    eprintln!("bench-large: indexing ...");
    indexer.wait();
    let index_time = opened.elapsed();
    let rate = file.size() as f64 / index_time.as_secs_f64() / 1e9;
    report.add(
        "Full index",
        format!("{} ({rate:.2} GB/s)", seconds(index_time)),
    );
    report.add("Footprint after indexing", mib(footprint()));

    let rows = source.row_count();
    let columns = source.column_count();
    if rows == 0 || columns == 0 {
        return Err("the generated file has no readable rows".to_owned());
    }
    if rows.saturating_mul(columns) < MAX_EDITS {
        return Err(format!(
            "the generated file has {} cells; the benchmark needs at least {MAX_EDITS}",
            rows.saturating_mul(columns)
        ));
    }

    let mut random = SplitMix64::new(RANDOM_SEED);
    let mut jumps = Vec::with_capacity(RANDOM_READS);
    for _ in 0..RANDOM_READS {
        let row = random.below(rows);
        let started = Instant::now();
        let _ = source.cell(row, SAMPLE_COLUMN.min(columns - 1));
        jumps.push(started.elapsed());
    }
    jumps.sort_unstable();
    report.add(
        "Jump to a random row (1000 reads)",
        format!(
            "p50 {}, p99 {}",
            milliseconds(jumps[RANDOM_READS / 2]),
            milliseconds(jumps[RANDOM_READS * 99 / 100])
        ),
    );

    let scroll_rows = SCROLL_ROWS.min(rows);
    let start = if rows > scroll_rows {
        random.below(rows - scroll_rows)
    } else {
        0
    };
    let (hits_before, misses_before) = source.cache_stats();
    let scrolled = Instant::now();
    for row in start..start + scroll_rows {
        for column in 0..columns {
            let _ = source.cell(row, column);
        }
    }
    let scroll_time = scrolled.elapsed();
    let (hits_after, misses_after) = source.cache_stats();
    let hits = hits_after - hits_before;
    let lookups = hits + (misses_after - misses_before);
    let hit_rate = if lookups == 0 {
        0.0
    } else {
        100.0 * hits as f64 / lookups as f64
    };
    report.add(
        format!(
            "Scroll {} rows x {columns} cells",
            compact_rows(scroll_rows)
        ),
        format!(
            "{}, block cache hit rate {hit_rate:.1}%",
            milliseconds(scroll_time)
        ),
    );

    for _ in 0..SAMPLE_READS {
        let _ = source.cell(random.below(rows), FOOTPRINT_SAMPLE_COLUMN.min(columns - 1));
    }
    report.add("Footprint after 10k random reads", mib(footprint()));

    for count in EDIT_COUNTS {
        eprintln!("bench-large: saving with {count} edits ...");
        let edits = apply_edits(&source, count, &mut random)?;
        let output = directory.path().join("saved.csv");
        let snapshot = source.save_snapshot();
        let cancelled = AtomicBool::new(false);
        let ((outcome, elapsed), peak) = peak_footprint(|| {
            let started = Instant::now();
            let written = write(&snapshot, &output, SAVE_BUFFER_BYTES, |_| {}, &cancelled);
            (written, started.elapsed())
        });
        let written =
            outcome.map_err(|error| format!("the save with {count} edits failed: {error:?}"))?;
        let verification = verify::saved(
            &output,
            &path,
            &source,
            &configuration,
            &edits,
            written,
            &mut random,
        )?;
        report.add(
            format!("Save, {count} edits"),
            format!(
                "{}, peak footprint {}; {}",
                seconds(elapsed),
                mib(peak),
                verification.summary()
            ),
        );
        source.overlay.remove_all();
        fs::remove_file(&output)
            .map_err(|error| format!("cannot remove {}: {error}", output.display()))?;
    }

    report.print();
    Ok(())
}

/// Waits until the first index block is published, or the whole small file is
/// indexed.
fn wait_for_first_block(source: &LargeCSVSource) {
    while source.row_count() < LargeFile::BLOCK_ROWS && !source.is_indexed() {
        std::thread::sleep(Duration::from_micros(100));
    }
}

/// Sets `count` distinct random cells and returns their text, keyed by
/// physical row.
fn apply_edits(
    source: &LargeCSVSource,
    count: usize,
    random: &mut SplitMix64,
) -> Result<HashMap<CellKey, String>, String> {
    let rows = source.row_count();
    let columns = source.column_count();
    let mut edits = HashMap::with_capacity(count);
    while edits.len() < count {
        let row = random.below(rows);
        let column = random.below(columns);
        let key = CellKey {
            row: source.physical_row(row),
            column,
        };
        if edits.contains_key(&key) {
            continue;
        }
        let text = format!("edited {}, \"quoted\"", edits.len());
        source
            .set_cell(&text, row, column)
            .map_err(|error| format!("cannot edit row {row}, column {column}: {error:?}"))?;
        edits.insert(key, text);
    }
    Ok(edits)
}

/// The metrics, printed as the Markdown table the Apple script emits.
#[derive(Default)]
struct Report {
    rows: Vec<(String, String)>,
}

impl Report {
    fn add(&mut self, metric: impl Into<String>, value: impl Into<String>) {
        self.rows.push((metric.into(), value.into()));
    }

    fn print(&self) {
        println!("| Metric | Value |");
        println!("|---|---|");
        for (metric, value) in &self.rows {
            println!("| {metric} | {value} |");
        }
    }
}

//! Ported from `LargeFileSaverTests.swift`, plus the block, header, cancel and
//! empty-file cases the plan calls out.

mod support;

use std::fs;
#[cfg(unix)]
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use support::{SplitMix64, TempDirectory};
use tabtool_core::{CsvConfiguration, CsvError, CsvTable, CsvWriter, Encoding};
use tabtool_large::{
    CellKey, CellOverlay, LargeCSVSource, LargeFileIndexer, LargeFileSaveError, LargeFileSaver,
    LargeSaveSnapshot, MappedFile, PreparedSave, save, volume_capacity, write,
};

const STEP_BYTES: usize = 4096;
/// One tebibyte; ample for every fixture here.
const AMPLE_CAPACITY: u64 = 1 << 40;

fn configuration() -> CsvConfiguration {
    CsvConfiguration::default()
}

fn bytes(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

fn make_source(
    directory: &TempDirectory,
    text: &[u8],
    configuration: &CsvConfiguration,
) -> LargeCSVSource {
    let path = directory.file("source.csv", text);
    let file = Arc::new(MappedFile::open(&path).expect("the source opens"));
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&file),
        configuration.clone(),
        STEP_BYTES,
        Duration::ZERO,
    ));
    indexer.start(|_| {});
    indexer.wait();
    LargeCSVSource::new(
        file,
        configuration.clone(),
        indexer,
        CellOverlay::new(),
        Vec::new(),
    )
}

/// A source whose indexer has not started, so its index is empty.
fn unstarted_source(
    directory: &TempDirectory,
    text: &[u8],
    configuration: &CsvConfiguration,
) -> LargeCSVSource {
    let path = directory.file("source.csv", text);
    let file = Arc::new(MappedFile::open(&path).expect("the source opens"));
    let indexer = Arc::new(LargeFileIndexer::new(
        Arc::clone(&file),
        configuration.clone(),
        STEP_BYTES,
        Duration::ZERO,
    ));
    LargeCSVSource::new(
        file,
        configuration.clone(),
        indexer,
        CellOverlay::new(),
        Vec::new(),
    )
}

/// Saves `source` to `out.csv` and returns the output bytes.
fn saved(source: &LargeCSVSource, directory: &TempDirectory, progress: impl Fn(f64)) -> Vec<u8> {
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let count =
        write(&source.save_snapshot(), &output, 64, progress, &cancelled).expect("the save writes");
    let data = fs::read(&output).expect("the output reads");
    assert_eq!(
        count,
        data.len() as u64,
        "the returned count is the output size"
    );
    data
}

fn save_with_capacity(
    snapshot: LargeSaveSnapshot,
    target: &Path,
    capacity: u64,
) -> Result<(), LargeFileSaveError> {
    let cancelled = AtomicBool::new(false);
    save(snapshot, target, |_| Some(capacity), |_| {}, &cancelled)
}

/// Prepares a save without installing it, so the two phases can be inspected.
fn prepare_with_capacity(
    snapshot: &LargeSaveSnapshot,
    target: &Path,
    capacity: u64,
) -> Result<PreparedSave, LargeFileSaveError> {
    let cancelled = AtomicBool::new(false);
    LargeFileSaver::prepare(snapshot, target, |_| Some(capacity), |_| {}, &cancelled)
}

/// The sorted names in `path`.
fn entries(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .expect("the directory reads")
        .map(|entry| {
            entry
                .expect("the entry reads")
                .file_name()
                .into_string()
                .expect("the name is UTF-8")
        })
        .collect();
    names.sort();
    names
}

#[test]
fn no_edits_is_byte_identical() {
    for text in [
        "a,b\n1,2\n3,4\n",
        "a,b\r\n1,2\r\n3,4\r\n",
        "a,b\n1,2\n3,4",
        "a,\"x\ny\"\r1,2\r",
        "\u{FEFF}a,b\n1,2\n",
        "",
    ] {
        let directory = TempDirectory::new();
        let source = make_source(&directory, text.as_bytes(), &configuration());
        assert_eq!(
            saved(&source, &directory, |_| {}),
            bytes(text),
            "input {text:?}"
        );
    }
}

#[test]
fn edits_only_change_edited_rows() {
    let directory = TempDirectory::new();
    let lines: Vec<String> = (0..200)
        .map(|row| format!("r{row},plain{row},\"q{row}\""))
        .collect();
    let source = make_source(
        &directory,
        format!("{}\n", lines.join("\n")).as_bytes(),
        &configuration(),
    );
    source
        .set_cell("changed", 70, 1)
        .expect("row 70 is editable");
    source.set_cell("x", 130, 0).expect("row 130 is editable");

    let mut expected = lines;
    expected[70] = "\"r70\",\"changed\",\"q70\"".to_string();
    expected[130] = "\"x\",\"plain130\",\"q130\"".to_string();
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes(&format!("{}\n", expected.join("\n")))
    );
}

#[test]
fn edited_row_keeps_its_terminator() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a\r\nb\nc\rd", &configuration());
    for row in 0..4 {
        source.set_cell("X", row, 0).expect("every row is editable");
    }
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes("\"X\"\r\n\"X\"\n\"X\"\r\"X\"")
    );
}

#[test]
fn edited_row_zero_keeps_bom() {
    let directory = TempDirectory::new();
    let source = make_source(
        &directory,
        "\u{FEFF}a,b\n1,2\n".as_bytes(),
        &configuration(),
    );
    source.set_cell("z", 0, 0).expect("row 0 is editable");
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes("\u{FEFF}\"z\",\"b\"\n1,2\n")
    );
}

#[test]
fn ragged_short_row_gets_padded() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b,c,d,e,f\nx,y\n", &configuration());
    source.set_cell("Z", 1, 5).expect("row 1 is editable");
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes("a,b,c,d,e,f\n\"x\",\"y\",\"\",\"\",\"\",\"Z\"\n")
    );
}

#[test]
fn quoted_multiline_neighbor_is_untouched() {
    let directory = TempDirectory::new();
    let text = "a,b\n\"x\ny, \"\"q\"\"\",2\nlast,row\n";
    let source = make_source(&directory, text.as_bytes(), &configuration());
    source.set_cell("L", 2, 0).expect("row 2 is editable");
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes("a,b\n\"x\ny, \"\"q\"\"\",2\n\"L\",\"row\"\n")
    );
}

#[test]
fn edited_row_with_quotes_off_is_written_plain() {
    let directory = TempDirectory::new();
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        escape_character: String::new(),
        ..CsvConfiguration::default()
    };
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration);
    source
        .set_cell("new", 1, 0)
        .expect("row 1 is editable without quotes");
    assert_eq!(saved(&source, &directory, |_| {}), bytes("a,b\nnew,2\n"));
}

#[test]
fn progress_reaches_one() {
    let directory = TempDirectory::new();
    let text = (0..500)
        .map(|row| format!("row{row},v"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = make_source(&directory, text.as_bytes(), &configuration());
    source.set_cell("e", 300, 0).expect("row 300 is editable");
    let seen = Mutex::new(Vec::new());
    let _ = saved(&source, &directory, |fraction| {
        seen.lock().expect("the progress list locks").push(fraction)
    });
    let values = seen.into_inner().expect("the progress list locks");
    assert_eq!(values.last(), Some(&1.0), "progress ends at one");
    assert!(
        values.windows(2).all(|pair| pair[0] <= pair[1]),
        "progress never goes backwards: {values:?}"
    );
}

/// A random fully quoted source is exactly what `CsvWriter` produces, so the
/// saver's output must equal the writer's output for the edited table.
#[test]
fn matches_in_memory_writer_on_random_edits() {
    const ALPHABET: &[&str] = &[
        "a", "b", "", " ", ",", "\"", "\n", "\r\n", "é", "日本", "x y", "\\",
    ];
    fn random_text(generator: &mut SplitMix64) -> String {
        (0..generator.below(4))
            .map(|_| ALPHABET[generator.below(ALPHABET.len())])
            .collect()
    }

    let configuration = configuration();
    for seed in 1..=60 {
        let mut generator = SplitMix64::new(seed);
        let columns = generator.below(6) + 1;
        let row_count = generator.below(300) + 1;
        let mut rows: Vec<Vec<String>> = (0..row_count)
            .map(|_| (0..columns).map(|_| random_text(&mut generator)).collect())
            .collect();
        let original = CsvWriter::new(
            &CsvTable::from_rows(rows.clone(), None, 0),
            None,
            &configuration,
        )
        .data()
        .expect("the fixture serializes");

        let directory = TempDirectory::new();
        let source = make_source(&directory, &original, &configuration);
        assert_eq!(source.row_count(), row_count, "seed {seed} row count");

        for _ in 0..generator.below(41) {
            let row = generator.below(row_count);
            let column = generator.below(columns);
            let text = random_text(&mut generator);
            source
                .set_cell(&text, row, column)
                .unwrap_or_else(|error| panic!("seed {seed}: edit rejected: {error:?}"));
            rows[row][column] = text;
        }
        let expected = CsvWriter::new(&CsvTable::from_rows(rows, None, 0), None, &configuration)
            .data()
            .expect("the edited table serializes");
        assert_eq!(saved(&source, &directory, |_| {}), expected, "seed {seed}");
    }
}

#[test]
fn two_edits_in_one_block_both_land() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n3,4\n5,6\n", &configuration());
    source.set_cell("x", 1, 1).expect("row 1 is editable");
    source.set_cell("y", 2, 0).expect("row 2 is editable");
    assert_eq!(
        saved(&source, &directory, |_| {}),
        bytes("a,b\n\"1\",\"x\"\n\"y\",\"4\"\n5,6\n")
    );
}

/// With a header enabled the display rows start at 1, but the saver still
/// addresses physical rows; an edit of physical row 0 writes the header row.
#[test]
fn edit_on_the_header_row_is_written_as_row_zero() {
    let directory = TempDirectory::new();
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..configuration()
    };
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration);
    let mut snapshot = source.save_snapshot();
    snapshot
        .edits
        .insert(CellKey { row: 0, column: 0 }, "H".to_string());
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let count = write(&snapshot, &output, 64, |_| {}, &cancelled).expect("the save writes");
    let data = fs::read(&output).expect("the output reads");
    assert_eq!(data, b"\"H\",\"b\"\n1,2\n");
    assert_eq!(count, data.len() as u64);
}

#[test]
fn unencodable_edit_fails_with_the_csv_error() {
    let directory = TempDirectory::new();
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..configuration()
    };
    let source = make_source(&directory, b"a,b\n", &configuration);
    let mut snapshot = source.save_snapshot();
    snapshot
        .edits
        .insert(CellKey { row: 0, column: 0 }, "日本".to_string());
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let error =
        write(&snapshot, &output, 64, |_| {}, &cancelled).expect_err("the text cannot be written");
    assert!(
        matches!(error, LargeFileSaveError::Csv(CsvError::UnencodableText)),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a failed write leaves no file");
}

#[test]
fn cancellation_stops_and_removes_partial_output() {
    let directory = TempDirectory::new();
    let text = (0..2000)
        .map(|row| format!("row{row},v"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = make_source(&directory, text.as_bytes(), &configuration());
    let snapshot = source.save_snapshot();
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let error = write(
        &snapshot,
        &output,
        16,
        |_| cancelled.store(true, Ordering::Release),
        &cancelled,
    )
    .expect_err("the save is cancelled");
    assert!(
        matches!(&error, LargeFileSaveError::Io(error) if error.kind() == io::ErrorKind::Interrupted),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a cancelled write leaves no file");
}

#[test]
fn cancelled_before_start_writes_nothing() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    let snapshot = source.save_snapshot();
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(true);
    let error =
        write(&snapshot, &output, 64, |_| {}, &cancelled).expect_err("the save is cancelled");
    assert!(
        matches!(&error, LargeFileSaveError::Io(error) if error.kind() == io::ErrorKind::Interrupted),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a cancelled write leaves no file");
}

#[test]
fn empty_file_writes_an_empty_file() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"", &configuration());
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let count = write(&source.save_snapshot(), &output, 64, |_| {}, &cancelled)
        .expect("the empty file writes");
    assert_eq!(count, 0);
    assert_eq!(
        fs::read(&output).expect("the output reads"),
        Vec::<u8>::new()
    );
}

#[test]
fn prepare_writes_beside_the_target_without_replacing_it() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let target = directory.path().join("source.csv");
    let original = fs::read(&target).expect("the source reads");

    let prepared = prepare_with_capacity(&source.save_snapshot(), &target, AMPLE_CAPACITY)
        .expect("the first phase writes the temporary file");

    assert_eq!(
        fs::read(&target).expect("the target still reads"),
        original,
        "prepare does not replace the target"
    );
    assert!(
        entries(directory.path())
            .iter()
            .any(|name| name.starts_with(".tabtool-save-")),
        "the temporary directory sits next to the target"
    );

    drop(prepared);
    assert_eq!(
        entries(directory.path()),
        ["source.csv"],
        "dropping the handle removes the temporary file"
    );
    assert_eq!(fs::read(&target).expect("the target still reads"), original);
}

#[test]
fn install_replaces_an_existing_file() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let target = directory.path().join("source.csv");
    let snapshot = source.save_snapshot();
    let prepared =
        prepare_with_capacity(&snapshot, &target, AMPLE_CAPACITY).expect("the save prepares");

    // The document layer releases its mapping between the phases; simulate
    // that here so the swap sees only the prepared handle.
    drop(snapshot);
    drop(source);
    prepared.install().expect("the prepared file is installed");

    assert_eq!(
        fs::read(&target).expect("the target reads"),
        b"a,b\n\"1\",\"9\"\n"
    );
    assert_eq!(entries(directory.path()), ["source.csv"]);
}

#[test]
fn install_creates_a_new_file() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let target = directory.path().join("copy.csv");
    let prepared = prepare_with_capacity(&source.save_snapshot(), &target, AMPLE_CAPACITY)
        .expect("the save prepares");

    prepared.install().expect("the new file is installed");

    assert_eq!(
        fs::read(&target).expect("the target reads"),
        b"a,b\n\"1\",\"9\"\n"
    );
    assert_eq!(entries(directory.path()), ["copy.csv", "source.csv"]);
}

#[test]
fn prepare_refuses_without_twice_the_free_space() {
    let directory = TempDirectory::new();
    let original = b"a,b\n1,2\n";
    let source = make_source(&directory, original, &configuration());
    let target = directory.path().join("source.csv");

    let error = prepare_with_capacity(
        &source.save_snapshot(),
        &target,
        (original.len() * 2 - 1) as u64,
    )
    .expect_err("the volume is too small");

    assert!(
        matches!(error, LargeFileSaveError::InsufficientDiskSpace),
        "unexpected error: {error:?}"
    );
    assert_eq!(entries(directory.path()), ["source.csv"]);
    assert_eq!(
        fs::read(&target).expect("the target is untouched"),
        original
    );
}

#[test]
fn install_keeps_the_swap_when_the_directory_sync_fails() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let target = directory.path().join("source.csv");
    let prepared = prepare_with_capacity(&source.save_snapshot(), &target, AMPLE_CAPACITY)
        .expect("the save prepares");
    drop(source);

    prepared
        .install_with_directory_sync(|_| Err(io::Error::other("injected sync failure")))
        .expect("a failed directory sync does not undo the swap");

    assert_eq!(
        fs::read(&target).expect("the target reads"),
        b"a,b\n\"1\",\"9\"\n"
    );
    assert_eq!(entries(directory.path()), ["source.csv"]);
}

#[test]
fn save_replaces_the_original() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let target = directory.path().join("source.csv");
    let snapshot = source.save_snapshot();
    // The document layer drops its source before saving so the mapping is
    // gone before the swap (Windows rejects replacing a mapped file).
    drop(source);
    save_with_capacity(snapshot, &target, AMPLE_CAPACITY).expect("the save replaces the file");
    assert_eq!(
        fs::read(&target).expect("the target reads"),
        b"a,b\n\"1\",\"9\"\n"
    );
}

#[cfg(unix)]
#[test]
fn saving_to_a_new_file_gives_it_the_default_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n", &configuration());
    let target = directory.path().join("copy.csv");
    let snapshot = source.save_snapshot();
    drop(source);
    save_with_capacity(snapshot, &target, AMPLE_CAPACITY).expect("the save writes a new file");
    let plain = directory.file("plain.csv", b"");
    let mode = |path: &Path| {
        fs::metadata(path)
            .expect("the file has metadata")
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(mode(&target), mode(&plain));
}

#[cfg(unix)]
#[test]
fn saving_over_a_file_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    let target = directory.path().join("source.csv");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600))
        .expect("the target becomes private");
    source.set_cell("9", 1, 1).expect("row 1 is editable");
    let snapshot = source.save_snapshot();
    drop(source);
    save_with_capacity(snapshot, &target, AMPLE_CAPACITY).expect("the save replaces the file");
    let mode = fs::metadata(&target)
        .expect("the target has metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "the target keeps its private mode");
}

#[test]
fn successful_save_leaves_no_temporary_directory() {
    let directory = TempDirectory::new();
    let output_directory = directory.path().join("out");
    fs::create_dir(&output_directory).expect("the output directory is created");
    let source = make_source(&directory, b"a,b\n", &configuration());
    let target = output_directory.join("copy.csv");
    let snapshot = source.save_snapshot();
    drop(source);
    save_with_capacity(snapshot, &target, AMPLE_CAPACITY).expect("the save writes");
    assert_eq!(entries(&output_directory), ["copy.csv"]);
}

#[test]
fn failed_save_leaves_no_temporary_directory() {
    let directory = TempDirectory::new();
    let output_directory = directory.path().join("out");
    fs::create_dir(&output_directory).expect("the output directory is created");
    let configuration = CsvConfiguration {
        encoding: Encoding::Windows1252,
        ..configuration()
    };
    let source = make_source(&directory, b"a,b\n", &configuration);
    let mut snapshot = source.save_snapshot();
    snapshot
        .edits
        .insert(CellKey { row: 0, column: 0 }, "日本".to_string());
    let target = output_directory.join("copy.csv");
    let error = save_with_capacity(snapshot, &target, AMPLE_CAPACITY)
        .expect_err("the edit cannot be written");
    assert!(
        matches!(error, LargeFileSaveError::Csv(CsvError::UnencodableText)),
        "unexpected error: {error:?}"
    );
    assert!(entries(&output_directory).is_empty());
    assert!(!target.exists());
}

#[test]
fn cancelled_save_leaves_the_target_untouched() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a,b\n1,2\n", &configuration());
    let target = directory.path().join("source.csv");
    let snapshot = source.save_snapshot();
    drop(source);
    let cancelled = AtomicBool::new(true);
    let error = save(
        snapshot,
        &target,
        |_| Some(AMPLE_CAPACITY),
        |_| {},
        &cancelled,
    )
    .expect_err("the save is cancelled");
    assert!(
        matches!(&error, LargeFileSaveError::Io(error) if error.kind() == io::ErrorKind::Interrupted),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        fs::read(&target).expect("the target reads"),
        b"a,b\n1,2\n",
        "a cancelled save leaves the original"
    );
    assert_eq!(entries(directory.path()), ["source.csv"]);
}

#[test]
fn save_fails_without_twice_the_free_space() {
    let directory = TempDirectory::new();
    let text = b"a,b\n1,2\n";
    let source = make_source(&directory, text, &configuration());
    let target = directory.path().join("source.csv");
    let snapshot = source.save_snapshot();
    drop(source);

    let error = save_with_capacity(snapshot.clone(), &target, (text.len() * 2 - 1) as u64)
        .expect_err("the volume is too small");
    assert!(
        matches!(error, LargeFileSaveError::InsufficientDiskSpace),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        fs::read(&target).expect("the target is untouched"),
        text,
        "a refused save leaves the original"
    );

    save_with_capacity(snapshot, &target, (text.len() * 2) as u64)
        .expect("exactly twice the size is enough");
}

#[test]
fn save_fails_while_indexing() {
    let directory = TempDirectory::new();
    let source = unstarted_source(&directory, b"a\nb\n", &configuration());
    let target = directory.path().join("source.csv");
    let error = save_with_capacity(source.save_snapshot(), &target, AMPLE_CAPACITY)
        .expect_err("the index is unfinished");
    assert!(
        matches!(error, LargeFileSaveError::IndexingNotFinished),
        "unexpected error: {error:?}"
    );
}

/// Windows cannot rewrite an open mapping (ERROR_USER_MAPPED_FILE), so this
/// failure is exercised on Unix only, like `MappedFileTests`.
#[cfg(unix)]
#[test]
fn save_fails_when_the_file_changed() {
    let directory = TempDirectory::new();
    let source = make_source(&directory, b"a\nb\n", &configuration());
    let target = directory.path().join("source.csv");
    fs::write(&target, b"short").expect("the target is rewritten");
    let snapshot = source.save_snapshot();
    drop(source);
    let error =
        save_with_capacity(snapshot, &target, AMPLE_CAPACITY).expect_err("the mapped file changed");
    assert!(
        matches!(error, LargeFileSaveError::SourceChanged),
        "unexpected error: {error:?}"
    );
}

/// Windows cannot truncate an open mapping (ERROR_USER_MAPPED_FILE), so this
/// failure is exercised on Unix only, like `MappedFileTests`.
#[cfg(unix)]
#[test]
fn truncation_during_the_copy_fails_the_save() {
    let directory = TempDirectory::new();
    let text = (0..2000)
        .map(|row| format!("row{row},v"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = make_source(&directory, text.as_bytes(), &configuration());
    let snapshot = source.save_snapshot();
    let target = directory.path().join("source.csv");
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let error = write(
        &snapshot,
        &output,
        1024,
        |_| {
            let file = OpenOptions::new()
                .write(true)
                .open(&target)
                .expect("the source opens for truncation");
            file.set_len(10).expect("the source is truncated");
        },
        &cancelled,
    )
    .expect_err("a short read fails the save");
    assert!(
        matches!(error, LargeFileSaveError::SourceChanged),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a failed write leaves no file");
}

/// The source scanner does not break rows at U+2028, but the block parser
/// does. An edit inserted into such a block must fail instead of writing a
/// row that belongs to another physical row.
#[test]
fn edit_in_a_unicode_line_break_block_is_refused() {
    let directory = TempDirectory::new();
    let rows: Vec<String> = (0..70).map(|row| format!("r{row},v{row}")).collect();
    let text = rows.join("\n").replace("r1,v1", "r1a\u{2028}r1b,v1");
    let source = make_source(&directory, text.as_bytes(), &configuration());
    let mut snapshot = source.save_snapshot();
    snapshot
        .edits
        .insert(CellKey { row: 1, column: 0 }, "x".to_string());
    let output = directory.path().join("out.csv");
    let cancelled = AtomicBool::new(false);
    let error =
        write(&snapshot, &output, 64, |_| {}, &cancelled).expect_err("the block is unaligned");
    assert!(
        matches!(error, LargeFileSaveError::Csv(CsvError::RowNotEditable)),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a refused write leaves no file");
}

/// Truncate after the first edited block has been written but before the next
/// edited block is read: the read must fail cleanly, not touch removed pages.
#[cfg(unix)]
#[test]
fn truncation_before_an_edited_block_read_fails_the_save() {
    let directory = TempDirectory::new();
    let text = (0..200)
        .map(|row| format!("row{row},v"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = make_source(&directory, text.as_bytes(), &configuration());
    source.set_cell("x", 70, 1).expect("row 70 is editable");
    source.set_cell("y", 130, 0).expect("row 130 is editable");
    let snapshot = source.save_snapshot();
    let target = directory.path().join("source.csv");
    let output = directory.path().join("out.csv");
    // The copy stops at the start of row 70 and reports exactly that fraction,
    // so truncating there leaves the block holding row 130 unread.
    let row_70_start: usize = text.lines().take(70).map(|line| line.len() + 1).sum();
    let size = text.len();
    let cancelled = AtomicBool::new(false);
    let error = write(
        &snapshot,
        &output,
        64,
        |fraction| {
            if fraction * size as f64 >= row_70_start as f64 {
                let file = OpenOptions::new()
                    .write(true)
                    .open(&target)
                    .expect("the source opens for truncation");
                file.set_len(10).expect("the source is truncated");
            }
        },
        &cancelled,
    )
    .expect_err("a truncated source fails the save");
    assert!(
        matches!(error, LargeFileSaveError::SourceChanged),
        "unexpected error: {error:?}"
    );
    assert!(!output.exists(), "a failed write leaves no file");
}

/// `File::create` would truncate the mapped source through a hard link or
/// symlink before any short-read guard could run. The identity probe must
/// refuse first and leave the source intact.
#[cfg(unix)]
#[test]
fn write_refuses_the_mapped_source_through_any_link() {
    let directory = TempDirectory::new();
    let original = b"a,b\n1,2\n";
    let source = make_source(&directory, original, &configuration());
    let path = directory.path().join("source.csv");
    let hard_link = directory.path().join("hard.csv");
    fs::hard_link(&path, &hard_link).expect("the hard link is created");
    let symlink = directory.path().join("link.csv");
    std::os::unix::fs::symlink(&path, &symlink).expect("the symbolic link is created");
    let snapshot = source.save_snapshot();
    let cancelled = AtomicBool::new(false);

    for target in [&path, &hard_link, &symlink] {
        let error = write(&snapshot, target, 64, |_| {}, &cancelled)
            .expect_err("writing to the source is refused");
        assert!(
            matches!(&error, LargeFileSaveError::Io(error)
                if error.kind() == io::ErrorKind::InvalidInput),
            "unexpected error for {target:?}: {error:?}"
        );
        assert_eq!(
            fs::read(&path).expect("the source still reads"),
            original,
            "the source is untouched after refusing {target:?}"
        );
    }
}

#[test]
fn volume_capacity_falls_back_to_the_nearest_existing_ancestor() {
    let directory = TempDirectory::new();
    assert!(
        volume_capacity(directory.path()).is_some(),
        "a temp directory has a volume"
    );
    assert!(
        volume_capacity(&directory.path().join("new.csv")).is_some(),
        "a new target reports its parent's volume"
    );
    assert!(
        volume_capacity(&directory.path().join("missing").join("new.csv")).is_some(),
        "a missing parent falls back to the nearest existing ancestor"
    );
    assert!(
        volume_capacity(Path::new("missing.csv")).is_some(),
        "a relative target falls back to the current directory"
    );
}

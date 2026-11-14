# TabTool universal (Rust)

`app/universal` is the Rust port of TabTool for Windows, Linux and Android.
The Apple app in `app/apple` stays the source of truth for behavior and looks;
this tree follows it and never invents a second product.

Plan 1 delivers the pure CSV core and the platform input proof. Plan 2 adds
the memory-mapped large-file layer. Plan 3 adds the document model, undo,
filters and saves. Plan 4 adds the desktop editor for Windows and Linux, with a
macOS development build that follows the Apple layout and behavior. Plan 5 adds
the Android UI and packaging. See [Plan 1 boundary](#plan-1-boundary),
[Plan 2 boundary](#plan-2-boundary), [Plan 3 boundary](#plan-3-boundary) and
[Plan 4 boundary](#plan-4-boundary).

## Workspace layout

```
app/universal/
├── Cargo.toml                 # workspace, resolver 3, edition 2024
├── Cargo.lock                 # committed
├── rust-toolchain.toml        # pinned Rust 1.99.1
├── crates/
│   ├── tabtool-core/          # pure CSV engine
│   ├── tabtool-large/         # mmap, scanner/index, sniffer, source, saver
│   ├── tabtool-document/      # document state, undo, filters and saves
│   ├── tabtool-i18n/          # 11-locale catalog and native language names
│   ├── tabtool-platform/      # platform traits, settings and OS services
│   ├── tabtool-settings/      # platform-neutral settings model
│   ├── tabtool-ui/            # design system, grid, chrome and kittest harness
│   └── tabtool-testdata/      # dev-only fixtures, manifest and generators
├── apps/
│   ├── tabtool-desktop/       # eframe app, document windows and `--smoke`
│   └── tabtool-android/       # cdylib, Gradle host, Kotlin bridge (spike)
├── xtask/                     # golden regeneration and check
├── testdata/
│   ├── fixtures/              # Apple fixtures, byte-for-byte
│   ├── fixtures.sha256.json   # SHA-256 manifest for every fixture
│   └── golden/                # six locale goldens and known divergences
└── docs/universal.md
```

`tabtool-core` owns the format configuration, the 18 encodings, the parser,
the writer and write validation, the detector, the table model and reorder
helpers, sorting, filters, clipboard helpers, export, numeric classification
and decimal-mark conversion. It has no UI, platform, async-runtime or `unsafe`
code (`#![forbid(unsafe_code)]`).

Plan 2 adds `tabtool-large`. Plan 3 adds `tabtool-document`. Plan 4 adds
`tabtool-i18n`, `tabtool-settings`, `tabtool-platform` and `tabtool-ui`, and
turns `tabtool-desktop` into the Windows/Linux editor with a macOS development
build. Plan 5 owns `tabtool-app`, Android navigation, SAF and packaging.

## Toolchain

| Tool | Version |
| --- | --- |
| Rust | 1.99.1 (pinned by `rust-toolchain.toml`), edition 2024, resolver 3 |
| `cargo-ndk` | 4.1.2 (Android builds) |
| Gradle | 8.14.7 (wrapper committed) |
| Android Gradle Plugin | 8.13.2 |
| Kotlin Gradle plugin | 2.4.21 |
| NDK | 28.2.13676358 |
| compileSdk / targetSdk / minSdk | 36 / 36 / 26 |
| JDK | 17–21 |
| Swift | Xcode 27.1 / Swift 6.4 (golden tool, macOS only) |

The desktop target also needs the platform GUI libraries: on Linux CI install
`libxkbcommon-dev` and `libwayland-dev` before building.

## Build and run

From `app/universal`:

```sh
cargo build
cargo run -p tabtool-desktop              # the desktop editor
cargo run -p tabtool-desktop -- --smoke   # prints "tabtool 0.1.0" and exits
cargo run -p tabtool-desktop -- FILE.csv  # open a file at launch
cargo build -p tabtool-desktop --release
```

`--smoke` is the headless check used by CI. It prints the core version and
returns without opening a window. The app opens one native deferred viewport
per document, an in-window menu bar in every viewport, and the root window is
shown only for the empty state or quit prompts. On Linux the app needs X11 or
Wayland plus the GUI libraries named under [Toolchain](#toolchain); no display
server is needed for `--smoke`.

The manual Windows/Linux launch, IME, dialog, drag/drop, 1 GiB large-file,
theme, language and accessibility pass is in
`docs/desktop-ui.md` in the repository root. Its visual baselines and measured
grid performance are recorded there and in
[Desktop UI (Plan 4)](#desktop-ui-plan-4).

## Tests

From `app/universal`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p tabtool-large               # fast large-file suite
cargo test -p tabtool-document            # document layer
cargo test -p tabtool-document --test parity  # ported-test inventory
cargo test -p tabtool-ui                   # widgets, grid, chrome, panels
cargo test -p tabtool-ui -p tabtool-desktop --tests  # UI scenarios
TABTOOL_RENDER_SNAPSHOTS=1 cargo test -p tabtool-desktop --test visual  # local images
cargo check -p tabtool-large --features bench   # compiles the benchmark binary
cargo test -p tabtool-core --test golden   # macOS only
```

The UI suites use `egui_kittest` for deterministic interaction and
accessibility assertions. Scenario, interaction and accessibility tests run on
Linux, Windows and macOS. Image snapshots are opt-in and local because they
depend on the operating system's fonts and GPU rasterizer:

- `crates/tabtool-ui/tests/snapshots/` holds the granular widget, grid and
  document-chrome image output when enabled;
- `apps/tabtool-desktop/tests/snapshots/` holds the consolidated Plan 4
  `<state>-<size>-<theme>.png` output from `tests/visual.rs` when enabled.

No PNG baseline is committed. Set `TABTOOL_RENDER_SNAPSHOTS=1` to render and
compare on the current machine, `UPDATE_SNAPSHOTS=1` to generate its baseline
after review. Without the flag the image tests print a skip notice and return.
Linux CI runs the scenario suite under `xvfb-run`; Windows and macOS run it
directly. `docs/desktop-ui.md` records the font setup, coverage warning and
snapshot policy.

The `bench` feature owns the benchmark binary and its dev-only dependencies,
so the plain workspace commands skip it. Every CI job runs the `cargo check`
above; local acceptance uses the same command after touching the benchmark.

The workspace suite covers the ported Apple core tests: the 18 encoding
choices with Foundation-derived guards for legacy code pages, the BOM and
trailing-code-unit cases exercised by the encoding oracle, parser error cases,
writer and validation rules, the 11 detector configurations, table invariants,
sorting, filters, clipboard, export and the fixture manifest. The table model is
covered by the ported unit tests, not by the Swift goldens.
`crates/tabtool-core/tests/roundtrip_property.rs` adds `proptest` round trips
over random tables (0–50 rows, 1–8 columns, headers on or off, cells containing
separators, quotes, backslashes, CR, LF, NEL, LS, PS and non-ASCII text) and a
no-panic parser check on arbitrary UTF-8.

A headerless table with no rows has no distinct serialized form: empty bytes
parse back as the documented 1×1 table. The property test pins that
normalization separately instead of pretending such a table round-trips.

`cargo test -p tabtool-core --test golden` is compiled only on macOS because
the expectations come from Foundation. On other hosts the command runs zero
tests; CI only invokes it on the macOS job.

The large-file suite covers mapped-file identity and short reads, the scalar
and block scanner paths and their parity, chunk boundaries, index publication
and cancellation, sniffer refusals, the block cache and overlay, header-row
mapping, and the saver's terminator, BOM, permission, free-space, cancel and
atomic-replace rules.

The document suite covers loading and phases, snapshots, editing and undo,
structural operations, format changes and transactional re-reads, revert,
clipboard and sorting, renaming, the filter model, saves and write gates, and
the large-document open, edit, save, adoption and cancel paths. Its cases are
ported from the Apple `CSVDocument*`, `RowFilterModel`, `FormatChangePlanner`
and `RevertCoordinator` tests. `crates/tabtool-document/tests/parity.rs` is the
checked inventory: it names the ported Swift files and cases, requires each
mapped Rust test function to exist, and records UI-only waivers and known gaps
with reasons. A renamed or removed Rust test fails that check.

## Large-file mode

Plan 2 adds `tabtool-large`, the layer that opens a CSV without reading it into
memory. Files at or above the large-file threshold (100 MiB = 104,857,600
bytes, `LargeFile::THRESHOLD_BYTES`) take this path; the threshold is a setting
in the app, and reading it plus routing between the in-memory and large paths
belong to the document layer (Plan 3). Smaller files keep the `tabtool-core`
path unchanged.

The crate has no document state and no UI. It provides the pieces the document
layer will use; [Plan 2 boundary](#plan-2-boundary) lists what stops here.

### Open and index

- `MappedFile::open` maps the whole file read-only and records its size,
  modification time and platform file key. Sequential scans use positioned
  reads into a reused buffer, so they do not fault the mapping in page by page;
  `with_bytes` serves random access from the mapping. Reads past the recorded
  size are clamped.
- `sniff` detects the format from the first 1 MiB
  (`LargeFile::SAMPLE_BYTES`), cut back to the last row break outside quotes,
  and parses that sample strictly. Large mode accepts UTF-8, MacRoman,
  CP1252/1250/1251/1253/1254, ISO Latin 2 and GBK. UTF-16/32, Shift_JIS,
  EUC-JP and ISO-2022-JP are refused (`UnsupportedLargeFileEncoding`), as are
  a separator, quote or escape that is not one ASCII byte or empty
  (`UnsupportedLargeFileDelimiter`).
- `LargeFileIndexer` scans on a background thread and publishes the growing
  `RowIndex` after every step. The first step is enough to show the first
  rows; rows whose end is not yet known stay hidden. The index keeps one
  checkpoint per 64 rows (`LargeFile::BLOCK_ROWS`), about 0.125 bytes per row.
  The indexer takes its step size as an argument; the benchmark and the
  planned document layer use 8 MiB steps.
- Rows break at LF, CR and CRLF outside quotes. U+0085 (NEL), U+2028 (LS) and
  U+2029 (PS) are not row separators in large mode, and the scanner does not
  support them as such. The strict parser that decodes a block does break at
  them, so a block containing one outside quotes decodes into more rows than
  the index recorded. Its rows cannot be mapped to physical rows without
  shifting, so that block is treated as unavailable and read-only: its cells
  stay empty and editing or saving it fails instead of writing shifted rows.
  Quote and escape handling follows parity and never stops the scan. A block
  that does not parse is shown as one raw-text cell per line when its decoded
  row count still matches the index; a mismatched block stays empty. Invalid
  UTF-8 shows U+FFFD and other encodings fall back to Latin-1. Neither lossy
  view can be edited.

### Rows, edits and reload

- `LargeCSVSource` implements `RowSource`. A cell read copies and decodes only
  its 64-row block and keeps it in a 64-block LRU cache; ragged rows are padded
  with empty cells. At most 2,048 columns (`LargeFile::MAX_COLUMNS`) are
  exposed; extra fields stay untouched in the file.
- Edits live in `CellOverlay`, keyed by physical file row and column, and win
  over the file's bytes. `set_cell` validates the text by trial serialization
  first, so ambiguous no-quote output and unencodable text fail before anything
  changes. It removes the override when the text equals the file's own text,
  and refuses while indexing, on non-editable rows, and when the block is not
  faithful (`RowNotEditable`).
- `set_header_enabled` changes only the mapping from display rows to physical
  rows. Edits stay attached to the physical row and the file's bytes do not
  change; undo for the toggle is document-layer work.
- `staleness()` returns `Unchanged`, `Appended` or `ReplacedOrTruncated`.
  Reads continue through the open descriptor after the file is replaced at its
  path, so rows already read stay visible. Edits are refused until the file is
  reloaded, and saving fails with `SourceChanged`. The Reload banner and the
  re-read are Plan 3 work.

### Saving

`save` refuses until the index covers the whole file (`IndexingNotFinished`),
then streams it. Unedited bytes are copied unchanged in 4 MiB positioned-read
chunks; only rows with edits are decoded, patched and written again. Each rewritten row keeps its own terminator (LF, CR or CRLF) and row 0
keeps the UTF-8 BOM. Rewritten rows go through `CsvWriter`, so quoting follows
the file's configuration and short rows are padded to the edited column. Every
other byte, including untouched quoting, ragged rows, terminators and the BOM,
survives.

`save` writes into a private temporary directory next to the target, flushes
the file to disk (and its parent directory on Unix), preserves an existing
target's permissions on Unix, and swaps the result in atomically (Unix
`rename`, Windows `ReplaceFileW`). It
needs at least twice the file's size free on the target volume
(`InsufficientDiskSpace`); a target that does not exist yet reports its
volume through its nearest existing ancestor. A failure or cancel before the
swap leaves the old file untouched and removes the temporary file.

A failed save can still have replaced the file. On Unix the parent directory
is synced after the atomic swap; if that sync fails, `save` returns an error
although the target already holds the new bytes. `Err` therefore does not
always mean "not saved": do not retry a failed save blindly. Reopen the target
and compare, or leave that decision to the document layer.

On Windows a mapped file cannot be replaced while a mapping of it is alive.
`save` consumes the snapshot and drops its own mapping before the swap; the
caller must also drop every other reference to the source, the indexer and the
mapping, then reopen the written file. If another mapping remains, the replace
fails with a typed I/O error and there is no in-place fallback.

`write` is the lower-level stream used by the benchmark: it refuses a path
that is the source or a hard link or symbolic link to it before truncating,
checks staleness at the start and before every edited-block read but not the
volume's free space, and removes its output on failure or cancel, including a
file that already existed at that path. It does not replace atomically.

The crate never saves on its own; the document layer decides when to call
`save` and keeps large files out of autosave.

### Limits

| Limit | Value |
| --- | --- |
| Large-file threshold | 100 MiB (104,857,600 bytes), a setting in the app |
| Rows per index block | 64 |
| Format sample | 1 MiB |
| Columns exposed | 2,048 |
| Cells in one copy | 1,000,000 (`LargeFile::MAX_COPY_CELLS`, enforced by the document layer) |

### What the layer allows

`tabtool-large` provides row and cell reads, the header toggle, cell edits in
the overlay, streaming save and the benchmark. It implements no structural
command, sort, filter, clipboard operation, export or decimal-mark conversion.

When the document layer and UI arrive, large mode keeps the Apple restrictions:
no Cut, Paste or Delete; no adding, deleting, moving, sorting or renaming rows
or columns; no filters, decimal-mark change, Export or Duplicate; column
reordering is off. Copy is capped at 1,000,000 cells. Those gates belong to the
document layer, not this crate.

### Plan 2 boundary

Delivered:

- `tabtool-large`: mapped file, byte scanner and row index, background
  indexer, format sniffer, block cache and overlay behind `RowSource`, and the
  streaming saver;
- the fast large-file tests in the workspace suite on Linux, Windows and macOS,
  and the benchmark from [Large-file benchmark](#large-file-benchmark).

Not in Plan 2:

- `tabtool-document`: `CSVDocumentLarge`, threshold settings, undo, save
  routing, format-change re-read, adoption after save and the staleness/reload
  banner;
- every screen and dialog: the grid, status line, toolbar, menu bar, banners,
  save panel and the large-window delegate;
- localization, packaging and installers.

Those screens and the large-file window are delivered by Plan 4.

## Document layer

Plan 3 adds `tabtool-document`, the UI-free document model above
`tabtool-core` and `tabtool-large`. One `Document` owns the table, phase,
selection, sort, filter, undo history and save state. Exactly one content mode
is active: an in-memory `Arc<CsvTable>`, a mapped `LargeState`, or neither
while nothing has loaded. The crate has no UI, platform code, async runtime or
`unsafe` code, and it never reads the operating-system locale.

### Phases and write gates

| Phase | Meaning | Editing | Save |
| --- | --- | --- | --- |
| `Loading` | Bytes are not parsed yet; the untitled placeholder is not saveable. | No | No |
| `Ready` | The table or mapped source is displayed. | Yes | Gated by the write issue |
| `ReadFailed(CsvError)` | Parsing failed; the table is empty. Change the format and re-read. | No | No |
| `UnreadableFile` | The bytes could not be read; the path stays bound for a retry. | No | No |
| `NeedsAttention(CsvWriteIssue)` | The table loaded but the current format cannot write it safely. | Yes | No |

The write issue is `UnencodableText` or `AmbiguousOutput`; it is recomputed
after every content change and undo, and it also drives `save_error`. A save
while the issue exists returns it as a typed `DocumentError::Csv` error and
never writes a partial file. `can_save` additionally requires a bound path, a
table, no re-read in flight and a `Ready` or `NeedsAttention` phase.
`save_as` works without an existing path; it binds the new path only after the
write succeeds.

The other document errors are `NotReady` (no table or no path), `Busy` (a
streaming large save owns the document), `RequiresConfirmation` (a dirty
re-read or an overlapping one) and `CannotWriteFile` / `CannotReadFile` for
I/O failures.

### Load, new documents and snapshots

- `Document::new` starts in `Loading` with the blank 1×3 placeholder table.
  `new_document` publishes a ready 1×3 table; with First Row as Header the
  three columns get empty headers. An untitled document stays dirty until
  `save_as` binds a path.
- `load_from_bytes` / `load_from_path` take an optional configuration: `None`
  detects the format, `Some` parses explicitly. A detection preference for
  Chinese is passed in; the crate never reads the system language.
- A parse error publishes `ReadFailed` with the typed `CsvError` and no table.
  An I/O error publishes `UnreadableFile`; the path stays bound, so a retry
  can re-read it. Success strips and remembers a UTF-8 BOM and publishes
  `Ready`.
- `snapshot` captures the table `Arc`, configuration and BOM flag for saving.
  It refuses `Loading`, `ReadFailed` and `UnreadableFile` documents.

### Undo actions and boundaries

Undo is an application-level stack owned by the document, never a system undo
manager. `UndoAction::key` is stable English text for the UI to map to a
localized name:

| Action | Key | Restores |
| --- | --- | --- |
| `EditCell` | `editCell` | The previous and next text of one cell |
| `AddRow` | `addRow` | A full state snapshot |
| `AddColumn` | `addColumn` | A full state snapshot |
| `DeleteRows` | `deleteRows` | A full state snapshot |
| `DeleteColumns` | `deleteColumns` | A full state snapshot |
| `MoveRows` | `moveRows` | A full state snapshot |
| `MoveColumn` | `moveColumn` | A full state snapshot |
| `PasteData` | `pasteData` | A full state snapshot |
| `ChangeFormat` | `changeFormat` | A full state snapshot |
| `ToggleHeader` | `toggleHeader` | A full state snapshot |
| `RenameColumn` | `renameColumn` | A full state snapshot |
| `SortRows` | `sortRows` | A full state snapshot |
| `RemoveSort` | `removeSort` | A full state snapshot |
| `RevertToSaved` | `revertToSaved` | A full state snapshot |

A full snapshot holds the table `Arc`, configuration, BOM flag, active sort and
selection; the table is shared, so an entry does not copy cells. One user
action is one step. A cell edit is targeted. A new action clears redo; a
no-op command records nothing and leaves redo alone, and a selection-only
change is neither a step nor a revision. Undo and redo bump the revision,
refresh the write issue and re-evaluate an applied filter. Structural changes
clear the active sort, except column moves, which keep it. Cell edits keep the
sort.

Large documents use targeted cell entries keyed by the physical file row and
header-toggle entries; their action names are the same `EditCell` and
`ToggleHeader`. A refused large undo or redo leaves the entry on its stack and
raises `DocumentNotice::EditRejected` with the typed error.

### Format changes, re-reads and revert

- `plan_format_change` maps a configuration diff to `None`, `DecimalMark`,
  `Header` or `Reparse`. Encoding, separator, quote and escape changes re-read
  the file; decimal-mark and header changes transform the table in memory. A
  parsing change wins when several fields change at once.
- A file-backed parsing change re-reads through `Storage`. A dirty document
  requires `force`; without it the call reports `RequiresConfirmation` and
  changes nothing. Only one re-read may run at a time; a second one is
  rejected.
- The re-read is transactional. The document captures its revision before
  reading, parses the bytes, and applies the candidate only while the revision
  is unchanged and the document is not loading, large or saving. An edit,
  format change, undo or Save As that lands during the parse therefore wins;
  the stale candidate is ignored.
- A failed read or parse keeps the previous table, configuration and undo
  history and records no step. A successful re-read replaces table,
  configuration and BOM with one `ChangeFormat` step (when the content
  changed), clears the sort, trims the selection and marks the document clean
  because it now matches disk. A document that had been read-failed can be
  recovered this way without an undo entry.
- `detect_format_from_disk` returns the detector's configuration without
  applying it; a large document sniffs the head of its mapping instead of
  reading the whole file.
- `set_decimal_mark` converts only cells that are numeric by the old mark,
  preserving leading zeros and exponents, and updates the configuration in one
  `ChangeFormat` step. `set_header_enabled` moves the first row across the
  header boundary with one `ToggleHeader` step: enabling makes row 0 the
  header and removes it from the data rows; disabling puts the header back as
  row 0. Renaming a column on a headerless table instead turns header mode on
  with generated names and keeps every row as a data row.
- `plan_revert` answers `Revert`, `NeedsConfirmation` or `Unavailable`.
  `revert_to_saved` re-reads the current path with the current configuration,
  records one `RevertToSaved` step and never writes the file. A failed read or
  parse keeps the previous state. A large document instead clears its overlay
  and undo history without re-reading; the header mapping stays as it is.

### Row filters

The `RowFilterModel` mirrors the Swift filter state machine:

- `criteria` is the draft being edited, `applied` is the last applied
  `RowFilter`, and `visible_rows` is the snapshot of model row indexes that
  pass it. Draft edits affect nothing until an apply.
- Apply-all combines every enabled criterion with AND. Apply-one uses only
  that criterion and enables it even when its checkbox was off. Clear removes
  the filter and resets the draft to one empty row. Add inserts after a
  criterion; remove always keeps one row. `fill_missing_columns` fills criteria
  without a column from a supplied column id; the filter bar must call it when
  it adds a row, because the document's apply path does not.
- Evaluation uses the document decimal mark for numeric comparisons and a
  fixed `en-US` locale for text, matching Foundation's locale-independent
  case-insensitive matching. The crate has no system locale.
- `visible_rows` is a snapshot: a cell edit never re-evaluates it, so a row
  does not vanish while it is being typed into. Undo/redo, sort, re-read,
  revert and structural changes refresh it. A refresh re-evaluates the applied
  filter, detaches draft criteria from deleted columns, drops applied criteria
  whose column is gone, and ends the filter when none is active. A reload that
  replaces every column identity remaps criteria by position when the column
  count is unchanged.
- Filters are view state: they are not undoable and do not bump the revision.
  An applied filter blocks row insertion, deletion and moves, and therefore
  also blocks paste, which inserts rows. Column edits and column deletion stay
  available. Sorting still applies to the whole table and then refreshes the
  visible set. Applying or refreshing trims the selection to visible rows.
- Export is UI work: the crate exposes `visible_table`, which contains only
  the visible rows with their edits, or the whole table without a filter. The
  export writer and its format options are Plan 4 desktop work and Plan 5
  Android work.
- A large document cannot filter and cannot show the filter bar.

### Saves

- Dirty is revision-based, matching the Rust design: `save` records
  `saved_revision = revision`, and any later content, format, structural or
  file-binding change bumps the revision. A sort that reorders rows also bumps
  it; a sort that only sets or changes the arrow without reordering records no
  undo step and does not bump it. Undoing back to the saved text still reports
  dirty. The document never re-reads the file to compare bytes, so a change
  made outside the document is not detected here; last writer wins. A large
  document is dirty while its overlay holds an edit, and becomes clean when an
  edit is written back to the file's own text.
- `can_save` requires a path, a table, no re-read in flight, a writable phase
  and no write issue. `save` writes the current path; `save_as` writes the new
  path, then binds it. A failed write leaves the previous file, path and dirty
  state untouched. `Storage::write_atomic` writes a temporary file and
  replaces atomically; a new file gets mode 0644 on Unix and an existing file
  keeps its permissions.
- Saving encodes a `DocumentSnapshot` before touching the file, so a gate
  failure cannot truncate anything. The snapshot cache is keyed by revision
  and holds an `Arc`; every revision bump clears it, and repeated saves of an
  unchanged document reuse it.
- Large documents never use the `Storage` save path. Their status is `None`,
  `Indexing(fraction)`, `Ready`, `Stale { appended }` or `Saving(fraction)`.

### Large documents

- `uses_large_mode(size, threshold)` opens a file large when
  `size >= threshold`; `open_file` routes by the file size, and a file below
  the threshold stays in memory. `open_large` maps and checks the format first;
  a successful below-threshold check returns `NotReady` without installing the
  source and leaves the content unchanged, but a mapping or format failure
  still publishes `UnreadableFile`/`ReadFailed` and binds the path. Use
  `open_file` for size routing.
- Opening maps the file, checks an explicit format or sniffs the head, and
  starts the indexer on its worker thread (8 MiB steps, 100 ms progress
  reports). Rows are readable while the index grows; cell editing requires
  `LargeStatus::Ready`. A refused format publishes `ReadFailed`, a missing
  file `UnreadableFile`, both without a table.
- `poll_large_indexing` publishes `Indexing(fraction)` and then `Ready`. The
  caller decides the polling cadence; the UI uses 4 Hz.
- `refresh_large_staleness` reports `Stale { appended: true }` for a file that
  only grew and `Stale { appended: false }` for a replaced or truncated one.
  A stale document refuses edits and saving until it is reloaded.
  `reload_large` re-maps the path with the current format and keeps the
  overlay, selection and undo history. `reopen_large` with a new format
  requires confirmation when the overlay is dirty, then drops the overlay and
  undo; a refused format, including UTF-16, leaves the source unchanged.
- A cell edit goes to the source overlay, validated by trial serialization, so
  ambiguous output and unencodable text fail before anything changes. Writing
  the same text is a no-op. The undo entry is keyed by physical row, so it
  survives a header toggle. The header toggle is a row-mapping change with one
  `ToggleHeader` step; bytes and overlay are untouched. Revert clears the
  overlay and the undo history without re-reading.
- Copy produces TSV and is capped at 1,000,000 cells. A refusal (no source,
  nothing valid selected, or over the limit) reports
  `CsvError::RowNotEditable`; the limit case also raises
  `DocumentNotice::CopyLimitExceeded`.

### Large saves, adoption and cancel

- `start_large_save` requires a ready, fully indexed, non-stale document; a
  second start is refused, as are stale files and unfinished indexes, with a
  typed `LargeFileSaveError`. `save_large_blocking` is start plus wait, for
  tests and callers without a progress UI.
- The saver runs in two phases. `prepare` applies the gates, checks free space
  and writes the temporary file while the original mapping is still alive.
  Then the document gives up its `LargeState`: rows become unavailable,
  `can_edit_cells` is false and the status is `Saving(fraction)`. The worker
  drops the snapshot and the original mapping before `install` replaces the
  target, which is required on Windows.
- A successful save adopts a new mapping and a complete index of the written
  file, with the same overlay entries (minus the edits that were written),
  the same column identities and the retained undo history. Save As binds the
  new path only after success; the saved revision is recorded.
- A cancel or pre-install failure restores the original mapping from the saved
  row index with a seeded indexer (no rescan), and returns the same overlay,
  column identities and undo history. A plain save with no edits writes
  nothing and reuses the mapping.
- While a save runs, every document-replacing operation (`open_file`,
  `open_large`, `load_*`, `new_document`, `reload_large`, `reopen_large`,
  `apply_parsing_configuration`, `revert_to_saved`, `save`, `save_as`)
  returns `DocumentError::Busy`. A second `wait_for_large_save` is a no-op;
  a `Saved` value obtained directly from `LargeSaveJob::wait` still owns the
  target mapping until it is dropped.
- Edits made during the save are not possible: the document releases the
  mapping when the save starts, so rows are unavailable while `Saving`. This
  is the Windows-safe behavior the design pins in §7.1. On adoption the
  pre-save overlay is carried over with the edits that were written removed;
  on cancel or pre-install failure the full pre-save overlay is restored.

### What the UI still owns

The document layer exposes state and commands; it schedules nothing and
renders nothing.

- **Autosave.** The UI saves 0.8 s after a change, on leaving the cell editor
  (mobile) and on backgrounding. It consults `is_dirty`, `can_save`,
  `write_issue` and `save_error`, and it never autosaves a large document.
  The crate has no timer.
- **Prompts and banners.** The UI asks before a dirty re-read, a dirty revert
  or closing; it renders the dirty dot, the reload banner, the typed error and
  `DocumentNotice`. `plan_revert`, `write_issue`, `save_error`,
  `large_status` and `take_notice` supply the decisions and values.
- **Locale and decimal separator.** The caller passes the system locale to
  sorting and the decimal separator to copy and paste. The document layer
  never reads either from the OS; filters use a fixed `en-US` locale because
  the matching is locale-independent.
- **Editor session and field commit.** The grid owns the text field. The UI
  commits the active cell before a sort, rename, filter apply/clear, export or
  save; the crate has no field editor and no notion of an uncommitted value.
- **Settings and presentation.** The large-file threshold is an argument
  (100 MiB default), not a setting read here. Save panels, menus, shortcuts,
  the first-responder chain, scroll position, selection rendering and
  accessibility are UI work (Plans 4–5).

### Known deviations and minors

- Opening a document is not a general "replace": the in-memory `load_from_path`
  (and therefore `open_file` for a small file) is a no-op when the document is
  already `Ready` or `NeedsAttention`, while `open_file` for a large file calls
  `open_large` and replaces it. The in-memory no-op keeps a late duplicate load
  from replacing edits. A caller that opens a different file starts a new
  `Document`.
- `open_large` on a below-threshold file maps and checks it before returning
  `NotReady`; the successful case leaves the content unchanged, but a read or
  format failure changes the phase and binds the path. Use `open_file` for size
  routing.
- Large copy has no distinct "nothing selected" error: every refusal is
  `CsvError::RowNotEditable`, and only the 1,000,000-cell limit also raises
  `CopyLimitExceeded`.
- A failed large open (`ReadFailed`, `UnreadableFile`) leaves no source but
  `can_edit_cells` answers true and the document reports dirty, because the
  path is bound and the revision moved. The phase is the authoritative block;
  the UI must not offer editing in that state.
- Dirty is revision-based, not a byte comparison. The Swift
  `hasUnsavedChangesTracksDiskBytes` behavior is deliberately not ported:
  external edits to an in-memory document are not detected, and undoing back
  to the saved text still reports dirty.
- The Rust load is synchronous and has no cancellation. The port keeps the
  "a cancelled or duplicate load never publishes" invariant with a no-op
  second load, not with a cancel API.
- A data-only failed load does not retain the supplied bytes. The caller can
  call `load_from_bytes` again with the same bytes; a file-backed document can
  retry from its bound path.
- If the written file is installed but cannot be re-indexed, the document
  reports the typed I/O error and has no source to restore, because the target
  already holds the new bytes. A pre-install failure restores the original
  state instead.
- `plan_format_change` handles a quote-character change, but no dedicated Rust
  test pins that single field; the parity inventory records it as a gap.

### Plan 3 boundary

Delivered:

- `tabtool-document`: phases, write gates, load and snapshots, editing,
  structural operations and undo, format changes and transactional re-reads,
  revert, clipboard, sorting, renaming, filters, small-file saves, large-file
  open and editing, and the two-phase large save with adoption and cancel;
- the document test suite and the checked parity inventory in
  `crates/tabtool-document/tests/parity.rs`.

Not in Plan 3:

- every screen and dialog: grid, status line, toolbar, menus, banners, save
  panels, autosave scheduling, prompts and the large-file window;
- localization, settings UI, packaging and installers;
- `tabtool-platform`, `tabtool-i18n`, `tabtool-ui` and `tabtool-app`.

Plan 4 supplies every screen and dialog listed above.

## Desktop UI (Plan 4)

Plan 4 turns `tabtool-desktop` into the TabTool editor for Windows and Linux
and keeps a macOS development build with the Apple layout and behavior. The
Apple app remains the visual source of truth; the reference screenshots are
`web/static/img/app-grid*.webp`, `app-columns*.webp` and `app-large*.webp`.

### Architecture

- `tabtool-i18n` owns the 11-locale catalog (`en`, `fr`, `es`, `de`, `it`,
  `pl`, `be`, `uk`, `zh-Hans`, `ja`, `ko`) and the native language names.
  Interface text comes from the catalog; the localization test scans the
  non-test sources of `tabtool-ui` and `tabtool-desktop` for hard-coded labels
  and unknown `tr` keys.
- `tabtool-settings` is a platform-neutral settings model. `tabtool-platform`
  wraps the OS services (dialogs, URLs, locale, clipboard support, settings
  paths) behind traits so tests use fakes.
- `tabtool-ui` contains the design tokens, widgets, virtualized grid, document
  chrome, filter bar, Settings form, About content and export panel. It has no
  platform services and no `unsafe`.
- `tabtool-desktop` owns the eframe application, one deferred viewport per
  document, menu dispatch, shortcuts, file workflows, autosave and the
  Settings/About windows. It is thin: document mutations go through
  `tabtool-document`, and every screen composes `tabtool-ui`.

### Shell and windows

- The root viewport shows the empty state ("No Documents") or quit prompts.
  Each document is a native deferred viewport with a 640×400 minimum; the
  Settings and About windows are deferred viewports too. The menu bar is drawn
  inside each viewport (a documented deviation from the macOS system menu bar).
- `FileWorkflow` handles New, Open, Open Recent, Save, Save As, Revert, Export,
  close prompts and quit. Autosave waits 800 ms after a change and never
  autosaves large files. The macOS Duplicate command is not implemented.
- A large file opened at or above the configured threshold uses the mapped
  source. The window shows indexing progress, the large-mode explanation, a
  stale-file Reload banner and a streaming-save progress bar with Cancel.

### Grid and chrome

- The grid is virtualized by row and column: a frame reads only the visible
  cells plus one row of overscan, and the row-number gutter sticks to the left
  while the header sticks to the top.
- Selection supports rows, columns and ranges; the focused cell draws the 2 pt
  focus ring. Arrow keys, Tab/Shift+Tab, Enter, Escape, Home/End and the
  platform select-all modifier are dispatched through one action enum.
- Cells edit in place through egui's `TextEdit`, so the platform IME composes
  Latin, Cyrillic and CJK text. An IME commit does not commit the cell on the
  same Enter. Column widths resize by dragging the header edge.
- The context menus disable unavailable commands instead of hiding them.
  Structural commands, sorting and filtering obey the document capabilities:
  filtered rows cannot be inserted or moved, and large mode disables
  structural edits, sorting, filters, Export, decimal-mark changes and Select
  All.
- The toolbar, format bar, filter bar, status line, needs-attention banner,
  save error, format alert, reparse confirmation and read-error overlay are
  the ports of the Apple chrome. Toolbar and format-bar strings come from the
  catalog.

### Files, export and preferences

- Open and Save As use the platform file dialogs; dropping files on a window
  opens them. Export writes a converted copy through the export panel and
  confirms an ambiguous decimal conversion.
- Settings has the four Apple tabs (General, New Document, Grid, Large Files)
  with the documented defaults and immediate application. About shows the
  version, descriptions, website, review link and the bundled MIT license.
- Language switching re-renders every open view in the same tick. Native file
  dialogs are created after the switch and use the platform locale.

### Fonts

The desktop UI ships no font files. `tabtool-ui` installs the operating
system's generic sans-serif, serif and monospace faces in a fixed fallback
order and lets `egui_system_font_provider` find a font for each script the
installed faces lack. Fonts are memory-mapped from the OS, never embedded, and
there is no bundled emoji font. On the first frame the app checks Chinese,
Japanese, Korean, Cyrillic, Greek and emoji samples and prints one stderr
warning per missing script, naming the script and the affected UI language.
The warning does not stop startup. The setup and coverage rules are in
`docs/desktop-ui.md`.

### Visual and performance verification

- `apps/tabtool-desktop/tests/visual.rs` renders 22 states at 1200×800 and
  640×400 in light and dark: the empty root, new document, CSV header, row and
  column selection, cell editor, format bar, filter bar, needs-attention,
  read-error, save-error, format-alert, reparse, large indexing, saving,
  stale, the four Settings tabs, About and Export. That is 88 local images
  when `TABTOOL_RENDER_SNAPSHOTS=1` is set; no baseline is committed.
- The granular UI image tests write under `crates/tabtool-ui/tests/snapshots/`
  with the same opt-in flag. Images are never a CI gate; scenario,
  interaction and accessibility tests run on Linux, Windows and macOS.
  `UPDATE_SNAPSHOTS=1` regenerates a local baseline after review.
- Grid virtualization is proven with a counting `RowSource`: at 640×400 a
  100,000-row scroll reads 119 cells (17 rows × 7 columns) in 78 µs in a
  release build on an Apple M4 Pro (1.36 ms in the debug test build); the
  large-document case reads 48 cells (16 rows × 3 columns) and never asks for
  a row past the published index. A partial publication of 120 of 5,000 rows
  reads the same 48 cells and never crosses the 120-row boundary. These are
  observations, not CI gates.
- The manual Windows/Linux parity checklist, known deviations, per-OS baseline
  plan and full performance notes are in `docs/desktop-ui.md`.

### Plan 4 boundary

Delivered: the desktop shell, grid, chrome, files/export, filters, settings,
About, language switching, visual baselines, performance tests and CI
coverage for Windows/Linux/macOS.

Not in Plan 4: the Android UI and navigation, SAF, share sheets, store
packaging, installers and signed distribution.

## Large-file benchmark

The large-file benchmark mirrors `scripts/bench-large` and the Performance
section of `README.md`. From `app/universal`:

```sh
cargo run -p xtask -- bench-large        # 5 GiB, the documented default
cargo run -p xtask -- bench-large 1      # 1 GiB local acceptance run
```

The task builds `bench-large` in release with the `bench` feature, creates a
deterministic 8-column CSV (quoted text, a multiline field every 1000 rows,
seed 42) in the system temporary directory, and removes it on success and on
failure. It refuses to start unless the temporary volume has at least three
times the requested size free, because the generated file and the saved copy
live side by side.

It prints a Markdown table with the same metrics as the Swift benchmark:

| Metric | Meaning |
| --- | --- |
| File | generated size, rows x 8 columns, generation time |
| Footprint before open | process memory before the file is mapped |
| Open to first 64 rows | map, sniff, first index step, decode the block |
| Open to full index | full scan time and GB/s |
| Footprint after index | process memory after indexing |
| Jump to a random row (1000 reads) | p50 and p99 latency |
| Scroll 100k rows x 8 cells | elapsed time and block-cache hit rate |
| Footprint after 10k random reads | process memory after the sampled reads |
| Save, 0 edits / Save, 1 edit / Save, 1000 edits | stream time, peak footprint, verification result |

Each save is verified by reopening the output: no edits must be byte-identical,
edits must be present, and 10k sampled rows must match the source.

Footprint is the resident set size on Unix and the working set on Windows, read
through the safe `memory-stats` crate. The Apple table reports physical
footprint, the number Activity Monitor shows; RSS/working set and physical
footprint count memory differently, so the footprint columns are not directly
comparable. Compare Rust runs on the same machine instead.

The Apple reference run on a 5 GiB file (Apple M4 Pro, 24 GB, APFS SSD, file in
the page cache) reported: open to first 64 rows 45 ms, open to full index 1.97 s
(2.7 GB/s), 43 MiB after index, jumps p50 0.06 ms / p99 1.2 ms, scroll
179 ms at 99.8% cache hits, and saves of 3.8 s / 4.8 s / 6.2 s with peak
footprints of 46 / 46 / 36 MiB. The timing and throughput numbers are the
target for the Rust port on the same hardware; the memory numbers are not.
The 1 GiB run is the local acceptance because it finishes in seconds and needs
only 3 GiB free.

## Fixtures and golden parity

The 36 Apple CSV fixtures live in `testdata/fixtures` with the exact bytes and a
SHA-256 manifest in `testdata/fixtures.sha256.json`; the copy is checked against
`app/apple/TabToolKit/Tests/TabToolKitTests/Fixtures` whenever that tree is
present. The dev-only `tabtool-testdata` crate reads them and provides the two
ported generators (`make_benchmark_csv`, `generate_large_csv`). Test data is
never regenerated during a build, and `.gitattributes` stops Git from rewriting
line endings.

`testdata/golden` holds one file per locale (`en_US`, `de_DE`, `fr_FR`,
`zh_CN`, `ja_JP`, `ko_KR`) generated by `tools/swift-golden`, a macOS SwiftPM
executable that links the existing `TabToolKit` package. Each file pins
detection, parsed rows, writer bytes, sort orders, filter matches, clipboard
output, export bytes and the encoding oracle. The goldens cover the 34
`Reading/` and `Heuristic/` fixtures; the two `Performance/` fixtures are copied
and hashed but are not decoded in the golden run.

Regenerate all locales from `app/universal`:

```sh
cargo run -p xtask -- golden
```

The task invokes Swift once per locale with both `-AppleLocale` and
`-AppleLanguages`, because Foundation's collation follows the preferred
language while `-AppleLocale` pins the decimal separator. The direct command
for one locale, run from the repository root, is:

```sh
swift run --package-path tools/swift-golden swift-golden \
  app/universal/testdata/fixtures \
  app/universal/testdata/golden/en_US.json \
  -AppleLocale en_US -AppleLanguages '(en)'
```

Check the committed data without Swift:

```sh
cargo run -p xtask -- golden --check
```

Collation comes from ICU4X on every platform, so a small number of strings can
order differently from Apple's Foundation. Those cases are tracked in
`testdata/golden/known_divergences.json`; the Rust test asserts that each one
still differs from Foundation and still produces the recorded Rust order.
Regenerating on a different macOS or Foundation version can change the
collation expectations; review a diff as a behavior change instead of editing
goldens by hand. See `testdata/golden/README.md` for details.

## Android host and spike result

`apps/tabtool-android` is a `cdylib` plus a Gradle project. Its
`MainActivity : GameActivity` hosts eframe and handles Back; `TabToolBridge.kt`
exposes clipboard, SAF open, insets and a native text editor through JNI. It
does not expose a system language list or a Back callback to Rust; Back lives
in `MainActivity`.

The decisive spike finding: winit 0.30.13 drops GameActivity's `TextEvent`, so
an egui `TextEdit` never receives soft-keyboard text. The spike therefore
opens a real Android `EditText` over the egui control and returns the committed
text through JNI. The Kotlin/Rust API and the emulator evidence are in
`apps/tabtool-android/README.md`.

Build steps:

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export JAVA_HOME="<path to a JDK 17-21>"

cd app/universal/apps/tabtool-android/android
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build -p tabtool-android
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n app.shurco.tabtool/.MainActivity
```

The spike passed all seven checks on the `Medium_Phone_API_36.1` AVD
(Android 16, arm64, `-gpu host`): Latin, Cyrillic and Chinese text entry with
the Gboard IME; clipboard set/get with another app; SAF open returning a
`content://` URI; keyboard and rotation insets; and Back/rotation without data
loss or crash. Two environment notes from that run: the AVD's Vulkan driver
crashes during wgpu adapter setup, so the app pins wgpu to GL on Android, and
eframe's default fonts lack CJK glyphs (the payload itself was exact). This is
a spike app, not product UI.

## Plan 1 boundary

Delivered in this plan:

- the Rust workspace, pinned toolchain, CI and fixture/golden tooling;
- `tabtool-core` with Swift-golden parity for detection, parsing, writing,
  sorting, filtering, clipboard, export and the encoding oracle; the table
  model is covered by ported unit tests;
- the Android input/clipboard/SAF/insets spike.

Not in Plan 1 (later plans):

- large-file mode (`tabtool-large`: mmap, scanner, index, sniffer, overlay,
  streaming save), delivered by Plan 2; see [Large-file mode](#large-file-mode);
- the document layer (`tabtool-document`: load/save gates, undo, autosave,
  large document);
- any production UI, desktop or Android, including the system language list,
  Back handling outside `MainActivity`, and the full grid;
- localization and the generated string catalog;
- packaging, versioning and installers;
- the full encoding matrix beyond the oracle samples: the goldens pin the
  18 single-byte decode tables, a fixed encode sample set and named BOM/truncation
  cases, not every malformed multi-byte sequence.

## Layer rules

- Dependency direction: `core` ← `large` ← `document`; `document` ← `app`;
  `i18n`, `settings`, `platform` and `ui` ← `app`. App crates stay thin.
- `core`, `large`, `document` and `settings` contain no egui, winit, platform
  code or async runtime. `core` also forbids `unsafe`; `unsafe` is allowed
  only in the large-file mmap code and platform FFI, each block with a
  `// SAFETY:` comment. `tabtool-ui` contains no platform services.
- No user-visible strings in the core. Localization starts with the i18n plan.
- `app/apple` is read-only here. Behavior and visuals follow the Apple app;
  deliberate differences must be listed in the design's §14 and approved.
- `docs/csv-format.md` is the normative format contract; the Swift tests and
  the committed goldens are the executable contract. A parity difference is a
  bug until it is proven impossible and tracked as an approved divergence.

# TabTool universal (Rust)

`app/universal` is the Rust port of TabTool for Windows, Linux and Android.
The Apple app in `app/apple` stays the source of truth for behavior and looks;
this tree follows it and never invents a second product.

Plan 1 delivers the pure CSV core and the platform input proof. Plan 2 adds
the memory-mapped large-file layer. Neither plan is a usable editor yet: see
[Plan 1 boundary](#plan-1-boundary) and [Plan 2 boundary](#plan-2-boundary).

## Workspace layout

```
app/universal/
├── Cargo.toml                 # workspace, resolver 3, edition 2024
├── Cargo.lock                 # committed
├── rust-toolchain.toml        # pinned Rust 1.99.1
├── crates/
│   ├── tabtool-core/          # pure CSV engine
│   ├── tabtool-large/         # mmap, scanner/index, sniffer, source, saver
│   └── tabtool-testdata/      # dev-only fixtures, manifest and generators
├── apps/
│   ├── tabtool-desktop/       # desktop shell and `--smoke` entry point
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

Plan 2 adds `tabtool-large`. Later plans add the other crates from the design:
`tabtool-document`, `tabtool-platform`, `tabtool-i18n`, `tabtool-ui` and
`tabtool-app`, plus the real desktop and Android UIs.

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
`libxkbcommon-dev`, `libwayland-dev` and `libgtk-3-dev` before building.

## Build and run

From `app/universal`:

```sh
cargo build
cargo run -p tabtool-desktop              # empty desktop window
cargo run -p tabtool-desktop -- --smoke   # prints "tabtool 0.1.0" and exits
cargo build -p tabtool-desktop --release
```

`--smoke` is the headless check used by CI. It prints the core version and
returns without opening a window. The desktop app itself is only a shell in
Plans 1 and 2; it has no table, document model or file UI.

## Tests

From `app/universal`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p tabtool-large               # fast large-file suite
cargo check -p tabtool-large --features bench   # compiles the benchmark binary
cargo test -p tabtool-core --test golden   # macOS only
```

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

`tabtool-desktop` is still a shell; no user can open a large file through it
yet.

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
  `i18n`, `platform` and `ui` ← `app`. App crates stay thin.
- `core`, `large` and `document` contain no egui, winit, platform code or
  async runtime. `core` also forbids `unsafe`; `unsafe` is allowed only in the
  large-file mmap code and platform FFI, each block with a `// SAFETY:`
  comment.
- No user-visible strings in the core. Localization starts with the i18n plan.
- `app/apple` is read-only here. Behavior and visuals follow the Apple app;
  deliberate differences must be listed in the design's §14 and approved.
- `docs/csv-format.md` is the normative format contract; the Swift tests and
  the committed goldens are the executable contract. A parity difference is a
  bug until it is proven impossible and tracked as an approved divergence.

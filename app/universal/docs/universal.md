# TabTool universal (Rust)

`app/universal` is the Rust port of TabTool for Windows, Linux and Android.
The Apple app in `app/apple` stays the source of truth for behavior and looks;
this tree follows it and never invents a second product.

Plan 1 delivers the pure CSV core and the platform input proof. It is not a
usable editor yet: see [Plan 1 boundary](#plan-1-boundary).

## Workspace layout

```
app/universal/
├── Cargo.toml                 # workspace, resolver 3, edition 2024
├── Cargo.lock                 # committed
├── rust-toolchain.toml        # pinned Rust 1.99.1
├── crates/
│   ├── tabtool-core/          # pure CSV engine
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

Later plans add the other crates from the design: `tabtool-large`,
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
Plan 1; it has no table, document model or file UI.

## Tests

From `app/universal`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p tabtool-core --test golden   # macOS only
```

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

It prints the same Markdown table as `scripts/bench-large`: generation time,
open to the first 64 rows, full index time and throughput, random-jump p50/p99
over 1000 reads, a 100k-row scroll with the block-cache hit rate, footprint
after indexing and after 10k random reads, and saves with 0, 1 and 1000 edits.
Each save is verified by reopening the output: no edits must be byte-identical,
edits must be present, and 10k sampled rows must match the source. Footprint is
the resident set size on Unix and the working set on Windows, read through the
safe `memory-stats` crate.

The Apple reference run on a 5 GiB file (Apple M4 Pro, 24 GB, APFS SSD, file in
the page cache) reported: open to the first 64 rows 45 ms, full index 1.97 s
(2.7 GB/s), 43 MiB after indexing, jumps p50 0.06 ms / p99 1.2 ms, scroll
179 ms at 99.8% cache hits, and saves of 3.8 s / 4.8 s / 6.2 s with peak
footprints of 46 / 46 / 36 MiB. The Rust port is expected to land in the same
range on the same hardware; the 1 GiB run is the local acceptance because it
finishes in seconds and needs only 3 GiB free.

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

Not in this plan (later plans):

- large-file mode (`tabtool-large`: mmap, scanner, index, sniffer, overlay,
  streaming save);
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

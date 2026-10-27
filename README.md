# TabTool

A modern, independent CSV editor for macOS in Swift. The app opens CSV/TSV/text files, detects the format automatically, lets you edit the table, and saves/exports data in the selected encoding and with the selected delimiter.

The project is in development. The public name, bundle ID, signing team, and App Store icon have not been chosen yet; placeholders are used in the code. The release checklist is at the end of this file.

## Features

- Automatic detection of encoding, delimiter, quotes, escape character, decimal mark, and a first header row.
- Opening files by dropping them on the Dock icon, the app icon in Finder, or an open window (`.csv`, `.tsv`, `.txt`, `.tab`, `.dat`).
- Editing cells, adding and removing rows and columns (also from the right-click menus on a row or a column header), reordering rows with drag & drop and columns by dragging the header, sorting rows by a column (click the arrow in the column header, the header menu, or the Table menu), renaming a column (double-click its title, the header menu, or the Table menu; without a header it turns Header on and keeps all rows), and moving the selected rows or columns one step with `⌥⌘` arrows.
- Copying and pasting rows/columns as TSV, undo/redo for cells, structure, paste, and format changes.
- Large file mode (files of 100 MiB and more): the file is memory-mapped and indexed in the background, so rows appear at once and a 5 GB file with 77.7 million rows is fully indexed about 2.7 seconds after launch, using under 100 MB of memory. You can browse, jump with `⌘↑`/`⌘↓`, and edit cells with undo; structural commands, sorting, export, Duplicate and the decimal mark are unavailable. Save and Save As stream the file (only edited rows are rewritten; the 5 GB file saves in about 6 seconds with the status bar showing progress and a Cancel button); the file is never autosaved, and closing the window or quitting with unsaved edits asks Save / Don't Save / Cancel. Saving needs free disk space of about twice the file size. Supported encodings, the row-break and quoting rules, and what is unavailable are listed in [docs/csv-format.md](docs/csv-format.md#large-file-mode).
- Saving and exporting with a different delimiter, encoding, quotes, decimal mark, and header.
- CSV, TSV, and plain text formats; UTF-8/16/32, MacRoman, Windows Latin 1/2, GBK, ISO Latin 2, Cyrillic, Greek, Turkish, Japanese encodings.
- No formulas, styles, or charts — this is a simple CSV editor.
- App Store: sandbox, no network, analytics, or third-party SDKs.

## Requirements

- macOS 14 or later.
- Xcode 27.1 / Swift 6.4.
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) 2.46 (`brew install xcodegen`).
- Python 3 — only for the version script.

## Repository structure

The repository follows these conventions:

- `app/apple` — the macOS app: XcodeGen `project.yml`, the `TabToolKit` package, the `Shared` shell, platform files `macOS/App`.
- `app/apple/TabToolKit` — CSV core: parser, writer, format heuristics, table model, clipboard, export. No AppKit/SwiftUI.
- `app/apple/Shared` — SwiftUI shell, document, grid on `NSTableView`, format panel, export, resources.
- `app/apple/macOS/App` — `Info.plist` (document types, category, encryption flag), entitlements, privacy manifest.
- `app/apple/Shared/Assets.xcassets` — placeholder `AppIcon`; final branding is a release task.
- `app/apple/Shared/Localizable.xcstrings` — String Catalog for all UI text; English only in v1.
- `app/apple/TabToolTests` and `app/apple/TabToolUITests` — app unit and UI tests.
- `scripts` — `app` (XcodeGen + xcodebuild), `test`, `bench-large` (large file benchmark), `lib/version`.
- `docs` — architecture, build, App Store, CSV format.
- `.superpowers/sdd` — working spec/plan artifacts, not tracked by git.
- `TabTool.xcodeproj` is generated and not committed.

## Build

From the repository root:

```sh
./scripts/app run macos
./scripts/app build macos
./scripts/app archive macos
```

`run macos` builds and opens the app. `build macos` and `archive macos` build without signing (`CODE_SIGNING_ALLOWED=NO`); the archive is an unsigned verification build, not an upload-ready App Store archive. The script calls `xcodegen generate` itself.

Without the script:

```sh
cd app/apple
xcodegen generate
xcodebuild -project TabTool.xcodeproj -scheme TabTool \
  -destination 'platform=macOS' build
```

## Tests

```sh
cd app/apple/TabToolKit && swift test
./scripts/test
./scripts/test --ui
```

`./scripts/test` runs the core package and the app unit tests. `./scripts/test --ui` adds the XCUITest smoke suite; it needs the macOS automation permission for the test runner (or run it from Xcode). Current coverage is 229 core Swift Testing tests plus the opt-in large file benchmark and 10 performance tests, 217 app unit tests, and 16 UI tests. `./scripts/bench-large [GiB]` runs the large file benchmark (see Performance). Details are in [docs/building.md](docs/building.md).

## Performance

Core operations were measured in release mode on synthetic CSVs (numeric and text columns, quoted multiline fields): 100k rows x 20 columns (20 MB), 500k x 10 (44 MB), and 5k x 200 (10 MB). The generator is `makeBenchmarkCSV` in the core test support; the performance tests in `CSVPerformanceTests` use smaller sizes so the debug run stays fast. Times in ms, before -> after:

| Operation | 100k x 20 | 500k x 10 | 5k x 200 |
| --- | --- | --- | --- |
| Format detection | 388 -> 39 | 836 -> 89 | 204 -> 19 |
| Write / non-UTF-8 save validation | 543 -> 152 | 1279 -> 268 | 282 -> 78 |
| Unquoted save validation (per edit) | 15* -> 1 | 793 -> 49 | 1 -> 0 |
| Convert decimal mark | 837 -> 354 | 2046 -> 868 | 432 -> 189 |
| Sort, numeric column | 94 -> 59 | 489 -> 309 | 4.5 -> 2.7 |
| Sort, text column | 384 -> 293 | 2239 -> 1793 | 15 -> 12 |

\* Stopped early at the first ambiguous cell; the full scan is the 500k figure.

Measured and left alone (already cheap or inherent): parsing (186 ms / 435 ms / 84 ms), building the table (20 / 83 / 4 ms), row reorder, move and delete (10-60 ms, linear), table equality (identical buffers return at once, 1.4 ms for distinct equal buffers), undo snapshots (copy-on-write; a cell edit copies one column, about 1 ms at 500k rows), and the grid (a cell edit refreshes only visible cells and never reloads the table; header sort arrows repaint only on change). Text sorting is bound by `localizedStandardCompare`.

### Large files

`./scripts/bench-large 5` (release build, Apple M4 Pro, 24 GB, APFS SSD, file in the page cache) on a generated 5.00 GiB CSV with 68.2 million rows x 8 columns (quoted text, a multiline field every 1000 rows). Footprint is the process's physical footprint, the number Activity Monitor shows; the benchmark process starts at 15 MiB.

| Metric | Value |
| --- | --- |
| Open to the first 64 rows (map, detect format, index the first 8 MiB, decode) | 45 ms |
| Full index | 1.97 s (2.7 GB/s) |
| Footprint after indexing | 43 MiB |
| Jump to a random row (1000 reads) | p50 0.06 ms, p99 1.2 ms |
| Scroll 100k rows x 8 cells | 179 ms, block cache hit rate 99.8% |
| Save, no edits / 1 edit / 1000 edits | 3.8 s / 4.8 s / 6.2 s; peak footprint 46 / 46 / 36 MiB |

In the Release app (sandboxed) with a 5.0 GiB file of 77.7 million rows: rows on screen and indexing under way 0.9 s after launch, fully indexed at 2.7 s, footprint 91 MB after indexing with no accessibility client; Save with one edit takes 6.3 s including indexing the written file, footprint up to 114 MB. Saves are disk-bound; cold-cache times were not measured.

Known limits: an accessibility client (VoiceOver, automation tools) reading the rows on screen near the end of a huge file makes AppKit allocate 8 bytes per row up to that row (about 620 MB at row 77.7 million). Rows near the top cost nothing.

## Documentation

- [Architecture](docs/architecture.md)
- [Build and versions](docs/building.md)
- [App Store and privacy](docs/app-store-compliance.md)
- [CSV format and editor behavior](docs/csv-format.md)

The local working spec/plan and execution log live in `.superpowers/sdd/` and are not part of git.

## Release checklist

The code is feature-complete for a 1.0 release, but the release identity is deliberately not chosen in the repository:

- [ ] Choose a unique public product name, bundle ID, and signing team. Update `app/apple/project.yml`, `app/apple/macOS/App/Info.plist`, and `docs/app-store-compliance.md`; replace `app.shurco.tabtool`.
- [ ] Replace the placeholder icon in `app/apple/Shared/Assets.xcassets/AppIcon.appiconset` with final 1024 px macOS artwork.
- [ ] Set `app/apple/version.json` (`name`, `build`) and sign with the team. Archive with Xcode Organizer (Product > Archive) or an explicit `xcodebuild archive` command without `CODE_SIGNING_ALLOWED=NO`; `./scripts/app archive macos` is an unsigned verification build.
- [ ] Inspect the archive: three document types in `Info.plist`, sandbox and user-selected read-write entitlements, `PrivacyInfo.xcprivacy` in `Contents/Resources`, and the app icon.
- [ ] Run `./scripts/test`, `./scripts/test --ui`, and the manual QA matrix in `docs/building.md`.
- [ ] Fill in App Privacy as **Data Not Collected**; confirm `ITSAppUsesNonExemptEncryption=false` with the account owner.
- [ ] Prepare screenshots, description, keywords, and reviewer notes without promising formulas, databases, or "any tables"; include the required MIT notice.
- [ ] After upload, pin the uploaded build in `app/apple/version.lock.json` and never reuse a build number.

## License

[MIT](LICENSE). The required copyright and permission notice is kept in the LICENSE file and bundled with the app.

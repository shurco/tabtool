# TabTool

A modern, independent CSV editor for macOS in Swift. The app opens CSV/TSV/text files, detects the format automatically, lets you edit the table, and saves/exports data in the selected encoding and with the selected delimiter.

The project is in development. The public name, bundle ID, signing team, and App Store icon have not been chosen yet; placeholders are used in the code. The release checklist is at the end of this file.

## Features

- Automatic detection of encoding, delimiter, quotes, escape character, decimal mark, and a first header row.
- Editing cells, adding and removing rows and columns (also from the right-click menus on a row or a column header), reordering rows with drag & drop and columns by dragging the header, sorting rows by a column (click the arrow in the column header, the header menu, or the Table menu), and moving the selected rows or columns one step with `⌥⌘` arrows.
- Copying and pasting rows/columns as TSV, undo/redo for cells, structure, paste, and format changes.
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
- `scripts` — `app` (XcodeGen + xcodebuild), `test`, `lib/version`.
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

`./scripts/test` runs the core package and the app unit tests. `./scripts/test --ui` adds the XCUITest smoke suite; it needs the macOS automation permission for the test runner (or run it from Xcode). Current coverage is 125 core Swift Testing tests plus 5 performance tests, 147 app unit tests, and 16 UI tests. Details are in [docs/building.md](docs/building.md).

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

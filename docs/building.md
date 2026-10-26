# Build

Requires macOS 14, Xcode 27.1 / Swift 6.4, [XcodeGen](https://github.com/yonaskolb/XcodeGen) 2.46, and Python 3 for the version script.

## Project

The source of truth is `app/apple/project.yml`. The `app/apple/TabTool.xcodeproj` file is generated and not tracked by git:

```sh
cd app/apple
xcodegen generate
```

`project.yml` defines three targets — the `TabTool` app, the `TabToolTests` unit tests, and the `TabToolUITests` UI tests — plus the `TabTool` scheme. The local `app/apple/TabToolKit` package is wired in as a Swift Package; the Xcode project contains no core sources. The `1.0.0` / `1` version in `project.yml` is a fallback in case `version.json` is unavailable.

The development bundle ID is `app.shurco.tabtool`. The public bundle ID, signing team, and app name are chosen before the App Store; they must be changed in `project.yml`, `Info.plist`, `docs/app-store-compliance.md`, and `version.json`. `project.yml` keeps `CODE_SIGN_IDENTITY=-` so local Debug builds and the verification archive need no team; a release build overrides the identity, team, and provisioning in Xcode or on the archive command line rather than committing signing material.

The app target reads `macOS/App/Info.plist`, signs with `macOS/App/TabTool.entitlements`, and bundles `macOS/App/PrivacyInfo.xcprivacy`. `ASSETCATALOG_COMPILER_APPICON_NAME=AppIcon` points at the placeholder asset catalog in `Shared/Assets.xcassets`. `ENABLE_TESTABILITY=YES` is Debug-only.

## Scripts

From the repository root:

```sh
./scripts/app run macos
./scripts/app build macos
./scripts/app archive macos
./scripts/app clean
./scripts/app version
```

- `run macos` — `xcodegen generate`, build, and open the app.
- `build macos` — build without signing (`CODE_SIGNING_ALLOWED=NO`).
- `archive macos` — Release archive in `app/apple/build/TabTool.xcarchive`. The script passes `CODE_SIGNING_ALLOWED=NO`, so it produces a verification archive without a team. For the App Store, archive with the team signature (Xcode Organizer or an xcodebuild invocation without that flag).
- `clean` — removes `app/apple/build`.
- `version` — shows the version from `app/apple/version.json`.

`./scripts/test` runs the core and app unit tests. `./scripts/test --ui` additionally runs the XCUITest smoke suite; a terminal process needs the macOS automation permission, otherwise run the UI scheme from Xcode. The UI tests open and close the app, so do not run them while manually testing the same build.

## Versions

The store version lives in `app/apple/version.json`, separately for macOS:

```json
{
  "macos": { "name": "1.0.0", "build": "1" }
}
```

`name` is substituted into `MARKETING_VERSION`, `build` into `CURRENT_PROJECT_VERSION`. Show and bump:

```sh
./scripts/app version
./scripts/app version bump macos
./scripts/app version bump macos patch
./scripts/app build macos --bump-version=patch
```

`bump` without a kind bumps only `build`. `patch`, `minor`, `major` move semver and also bump `build`. The bump kind is given only as `--bump-version=KIND`; a bare flag does not take the next word. After uploading to App Store Connect, the number is pinned in `app/apple/version.lock.json` (the file is created on the first upload).

## Tests

Core:

```sh
cd app/apple/TabToolKit
swift test
```

Everything at once from the root:

```sh
./scripts/test
```

The app directly:

```sh
cd app/apple
xcodegen generate
xcodebuild -project TabTool.xcodeproj -scheme TabTool \
  -destination 'platform=macOS' -only-testing:TabToolTests test
```

UI smoke suite:

```sh
./scripts/test --ui
# or
cd app/apple
xcodebuild -project TabTool.xcodeproj -scheme TabTool \
  -destination 'platform=macOS' -only-testing:TabToolUITests test
```

Test fixtures are in `app/apple/TabToolKit/Tests/TabToolKitTests/Fixtures`. These are synthetic CSVs plus new round-trip/edge-case files. The current counts are 115 core Swift Testing tests plus 4 XCTest performance tests, 126 app unit tests, and 14 UI tests. The UI suite opens a fixture through Launch Services and checks the grid and status line; the system open panel itself stays manual. The UI test resolves the app by the development bundle ID `app.shurco.tabtool`; update it together with the release bundle ID.

## Signing and App Store

App Sandbox and user-selected read-write are already in `app/apple/macOS/App/TabTool.entitlements`. There is no network, iCloud, or Apple Events.

For the App Store:

1. Choose the public name, bundle ID, and developer team.
2. Update `project.yml`, `Info.plist`, `TabTool.entitlements`, `docs/app-store-compliance.md`.
3. Archive with the team signature in Xcode Organizer (Product > Archive) or with an explicit `xcodebuild archive` command that does not set `CODE_SIGNING_ALLOWED=NO`. `./scripts/app archive macos` is an unsigned verification build only.
4. Upload the archive via Xcode Organizer or Transporter.
5. Fill in App Privacy: no data is collected. The privacy manifest is `app/apple/macOS/App/PrivacyInfo.xcprivacy`; it declares the User defaults required-reason API (`CA92.1`).
6. Check `version.json` and `version.lock.json` before every upload.

The icon and store metadata are a separate release step; a placeholder is used in development. Choose a unique name and icon.

## Manual QA matrix

The automated suites cover parsing, writing, encoding, detection, document save/re-read, undo, grid coordination, and the main UI controls. Before release, verify the following on a clean account:

| Area | Check | Automated |
|---|---|---|
| Encodings | UTF-8, UTF-8 BOM, UTF-16, CP1252, MacRoman, GBK open and save/reopen | Core tests; open/save panel manual |
| Separators | comma, semicolon, tab, pipe read/write and export | Core tests; panel manual |
| Quotes | doubled, backslash, none; quoted newline/separator/quote | Core tests |
| Numbers | dot/comma decimal conversion, copy/paste | Core + app tests; Numbers/TextEdit interoperability manual |
| Header | on/off detection, toggle, export header | Core + app tests |
| Errors | empty file, blank lines, ragged rows, trailing fields, invalid quote/encoding, unencodable edit, ambiguous no-quote edit, failed re-read | Core + app tests; dialog text manual |
| Editing | undo/redo of cell, edit, add/delete/move/paste/format; multi-row drag; column reorder | App tests; drag and focus manual |
| Files | open fixture, save, Save As, reopen, export/re-import | App tests for snapshots; panels manual |
| Scale | 100k-row file loads, scrolls, edits, saves | Performance tests; GUI smoke manual |
| Accessibility | VoiceOver reads the grid, toolbar, format bar, banner, and errors; full keyboard access reaches grid and format bar | Labels/identifiers committed; VoiceOver manual |
| Appearance | light/dark mode, minimum window size | Manual |

## Archive inspection

After `./scripts/app archive macos`:

```sh
plutil -p app/apple/build/TabTool.xcarchive/Products/Applications/TabTool.app/Contents/Info.plist
plutil -p app/apple/macOS/App/TabTool.entitlements
ls app/apple/build/TabTool.xcarchive/Products/Applications/TabTool.app/Contents/Resources
```

Expected: CSV/TSV/plain-text document types, `LSApplicationCategoryType=public.app-category.utilities`, `ITSAppUsesNonExemptEncryption=false`, `AppIcon` in `Contents/Resources`, and `PrivacyInfo.xcprivacy`. The unsigned archive cannot prove the effective codesign entitlements; verify those in Xcode or App Store Connect after selecting the team.

## Limitations

- The target macOS is 14.0. The new SwiftUI `Document` protocol (macOS 27) is not used; `ReferenceFileDocument`/`DocumentGroup` are soft-deprecated but available since macOS 11.
- `TabTool.xcodeproj` is not committed. If the Xcode project "disappears", run `xcodegen generate`.
- `swift test` in the package does not build the app; for UI and document use `xcodebuild test` or `./scripts/test`.

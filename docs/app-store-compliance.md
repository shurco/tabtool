# App Store and privacy

This document describes what is needed to publish TabTool in the App Store. It does not replace the [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/) or account rules.

## What the app does

TabTool is a simple CSV editor for macOS. It opens CSV/TSV/text files, detects the format, shows a table, lets the user edit cells/rows/columns, and save or export the file in the selected encoding and with the selected delimiter. There are no formulas, macros, network, accounts, or purchases.

## Sandbox and file access

`app/apple/macOS/App/TabTool.entitlements`:

- `com.apple.security.app-sandbox` — mandatory sandbox.
- `com.apple.security.files.user-selected.read-write` — access only to files selected in open/save panels.

There is no network, iCloud, Apple Events, camera, microphone, contacts, or other permissions. The app does not download or execute code. `ENABLE_HARDENED_RUNTIME=YES` is set in `app/apple/project.yml`; the development archive is unsigned only because no team is configured yet.

Large file mode (files at or above the large file threshold, 100 MiB by default) needs no further entitlement:

- Reading: the user-selected file is opened read-only and memory-mapped (`mmap`, `MAP_PRIVATE`) and read with `pread`, inside the document's security-scoped access. `stat`/`fstat` compare its size, modification time, and inode with the values at open to notice changes on disk.
- Save: the file is streamed into the item-replacement directory on the same volume (`FileManager.url(for: .itemReplacementDirectory, ...)`, allowed by the sandbox) and swapped in with `FileManager.replaceItemAt`. Before writing, the free space of the volume is checked (`volumeAvailableCapacityForImportantUsageKey`).
- Save As: `NSSavePanel` grants access to the chosen URL; the file is streamed there and the system document is moved to it.
- The system never autosaves a large document; nothing is written without Save, Save As, or the Save choice of the close/quit prompt.

## Build configuration

`app/apple/project.yml` is the source of truth:

- `INFOPLIST_FILE=macOS/App/Info.plist`;
- `CODE_SIGN_ENTITLEMENTS=macOS/App/TabTool.entitlements`;
- `ASSETCATALOG_COMPILER_APPICON_NAME=AppIcon`;
- `MACOSX_DEPLOYMENT_TARGET=14.0`, `SWIFT_VERSION=6.0`, `SWIFT_STRICT_CONCURRENCY=complete`;
- `ENABLE_TESTABILITY=YES` is Debug-only;
- `PrivacyInfo.xcprivacy` is copied into the app's resources.

`Info.plist` also carries `LSApplicationCategoryType=public.app-category.utilities`, `ITSAppUsesNonExemptEncryption=false`, `LSMinimumSystemVersion=$(MACOSX_DEPLOYMENT_TARGET)`, `NSPrincipalClass=NSApplication`, `CFBundleDevelopmentRegion=en`, and `CFBundleLocalizations` with the 11 interface language codes (`en`, `fr`, `es`, `de`, `it`, `pl`, `be`, `uk`, `zh-Hans`, `ja`, `ko`). The required MIT notice is bundled from the repository `LICENSE` file.

## Privacy

- Data is not collected, transmitted, or used for tracking.
- No analytics, Crashlytics, Firebase, ads, or third-party SDKs.
- App Privacy in App Store Connect: **Data Not Collected**.
- `app/apple/macOS/App/PrivacyInfo.xcprivacy`: `NSPrivacyTracking=false`, empty `NSPrivacyCollectedDataTypes`, empty `NSPrivacyTrackingDomains`, and three accessed-API entries. None of them changes the Data Not Collected answer. The file is copied into `Contents/Resources`.
  - `NSPrivacyAccessedAPICategoryUserDefaults`, reason `CA92.1`: the app stores only its own preferences in its own defaults domain: window and format-bar state, the Settings window values (new document format, grid, large file threshold), and the language override (`appLanguage`, mirrored to `AppleLanguages`). The one declaration covers all of them; no further manifest entry is needed for Settings.
  - `NSPrivacyAccessedAPICategoryFileTimestamp`, reason `3B52.1`: large file mode reads the size and modification time (`stat`, `fstat`, `contentModificationDateKey`) of the file the user opened or saved, to notice changes on disk.
  - `NSPrivacyAccessedAPICategoryDiskSpace`, reason `E174.1`: before saving a large file, the app checks that the volume has room for the rewritten copy and shows an error instead of starting the write.
- Local settings and unsaved data stay on the device; the app makes no network requests.
- `Shared/Localizable.xcstrings` contains the UI strings in English and 10 translations; there are no hidden network or analytics strings.

## Document types

In `Info.plist`:

| Type | UTI | Role | Extensions |
|---|---|---|---|
| CSV | `public.comma-separated-values-text` | Editor, Owner | `.csv` |
| TSV | `public.tab-separated-values-text` | Editor, Owner | `.tsv` |
| Plain text | `public.plain-text` | Editor, Alternate | `.txt`, `.tab`, `.dat` |

Plain text is Alternate so the app does not intercept all text files. Arbitrary extensions are not opened. Files dropped on a document window open as new documents only when the extension is one of the above (`DroppedFiles.tableExtensions`).

## Name, bundle ID, icon

Before release, the build must have:

- a public name (TabTool) that does not conflict with an existing listing;
- the bundle ID `app.shurco.tabtool`;
- a signing team and provisioning setup;
- its own icon and branding. The repository ships a blue table placeholder in `Shared/Assets.xcassets/AppIcon.appiconset` (16–1024 px); replace it with final 1024 px artwork at release.

Choose a unique name and icon. The required MIT copyright and permission notice is kept in `LICENSE` and bundled with the app.

## Category and store card

- Category: Utilities.
- Subtitle and description: a simple CSV editor for macOS, without promises of formulas, databases, or "any tables".
- Screenshots: empty document, opened CSV, format panel, export.
- Keywords: CSV, TSV, editor, table, delimiter, encoding.
- Languages: English, French, Spanish, German, Italian, Polish, Belarusian, Ukrainian, Simplified Chinese, Japanese, Korean. The 10 translations are machine-generated.

## Notes for Review

Briefly for the reviewer:

> TabTool is a local CSV/TSV/text editor. It reads a user-selected file, detects the delimiter/encoding/quote style, lets the user edit a grid, and saves or exports the result. It has no network access, no account, no in-app purchases, and collects no data. App Sandbox is enabled with user-selected read-write access only. Test files are included in the review notes/attachment.

If the reviewer needs a test file, attach a small CSV with commas, quotes, and a header; the format is detected automatically.

## Export encryption

The app does not implement its own cryptography and does not transmit data over the network. `ITSAppUsesNonExemptEncryption=false`; the account owner confirms the exemption basis.

## Before submitting

- [ ] Signing team is chosen; the name TabTool and bundle ID `app.shurco.tabtool` are checked against existing listings.
- [ ] Final icon replaces the placeholder in `Shared/Assets.xcassets/AppIcon.appiconset`.
- [ ] `version.json` carries the release version and build.
- [ ] `./scripts/test` and `./scripts/test --ui` pass.
- [ ] `./scripts/app archive macos` succeeds; inspect `Info.plist`, entitlements, `PrivacyInfo.xcprivacy`, and `AppIcon.icns` in the archive.
- [ ] The archive is signed with the team and validated in Xcode Organizer or Transporter.
- [ ] App Privacy: Data Not Collected.
- [ ] Native speakers have reviewed the machine-generated translations in `Shared/Localizable.xcstrings`.
- [ ] Screenshots and description without promises of formulas/databases.
- [ ] The required MIT notice is bundled with the app and kept in `LICENSE`.
- [ ] Sandbox, open/save, export, errors, dark mode, VoiceOver, and a 100k-row file verified on a clean system.
- [ ] Large file mode verified in the sandboxed build: open a file of several GB, edit a cell, Save, Save As to a new location, close with unsaved edits.
- [ ] `version.lock.json` updated after upload.

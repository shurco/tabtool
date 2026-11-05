# App Store and privacy

This document describes what is needed to publish TabTool in the App Store. It does not replace the [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/) or account rules.

## What the app does

TabTool is a simple CSV editor for macOS, iPhone, and iPad. It opens CSV/TSV/text files, detects the format, shows a table, lets the user edit cells/rows/columns, and save or export the file in the selected encoding and with the selected delimiter. There are no formulas, macros, network, accounts, or purchases. The iOS app uses the same display name and bundle ID.

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

## iOS

The iOS target is `TabToolIOS` in `app/apple/project.yml`. The product and the module are both `TabTool`, bundle ID `app.shurco.tabtool`, deployment target iOS 17, `TARGETED_DEVICE_FAMILY` 1 and 2.

- `INFOPLIST_FILE=iOS/App/Info.plist`. Document types match the Mac plist. `LSSupportsOpeningDocumentsInPlace` and `UISupportsDocumentBrowser` are set. There is no `NSPrincipalClass` and no `LSMinimumSystemVersion`.
- `CODE_SIGN_ENTITLEMENTS=iOS/App/TabTool.entitlements`. The file is an empty dictionary. iOS does not use the Mac sandbox entitlements (`com.apple.security.*`).
- `iOS/App/PrivacyInfo.xcprivacy` declares the same three accessed APIs as the Mac manifest: User defaults `CA92.1` (`@AppStorage` preferences), File timestamp `3B52.1`, and Disk space `E174.1`. The linked core calls the file and disk APIs. Tracking stays off and no data is collected.
- The app icon is the `ios` / `universal` 1024 entry in `Shared/Assets.xcassets/AppIcon.appiconset`, reusing `icon_512x512@2x.png`.
- `./scripts/app build ios` and `./scripts/app test ios` build and test the iOS Simulator. `./scripts/app archive ios` exits with an error; archive stays macOS-only.
- Platform differences: files open through the system document browser, not by dropping a file onto a window. Settings and About are a sheet. The language change asks the user to open TabTool again, because the app cannot relaunch itself. The grid selects one row or one column. A large file can be edited and saved; Save As asks for a folder. The system does not autosave it.

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

Paste this into App Store Connect > App Review Information > Notes (answers Guideline 2.1 "Information Needed"). Also attach a screen recording from a physical iPhone/iPad on the latest iOS: launch the app, create a new document, edit cells, add/delete a row and a column, sort, filter, Format sheet, export, Settings > About. There is no login, account deletion, user-generated content sharing, or paid content to show.

> 1. Purpose and audience. TabTool is a simple, local CSV/TSV/text table editor for iPhone and iPad (also on Mac). It solves the problem of opening and fixing delimited text files whose encoding, delimiter, or quoting is unknown or wrong: the app detects them automatically, shows a grid, lets the user edit cells, rows and columns, and saves or exports the file with a chosen delimiter and encoding. Audience: people who work with CSV exports (analysts, developers, accountants, data hobbyists). No formulas, macros, or charts.
>
> 2. How to use. No account or login is needed. Open the app, tap "+" > "New Document" (or "Import" to pick a file from Files). Tap a cell to select it, tap again to edit. Use the toolbar for Undo/Redo, Filters, Export, and the overflow menu for Format, Settings, About. Files are saved automatically in the app's Documents folder. A sample file is not required, but any small .csv with a header row, commas, and quoted fields works; the format is detected on open.
>
> 3. External services. None. The app makes no network requests and uses no third-party SDKs, analytics, ads, authentication, payment, or AI services. All processing is on the device. App Privacy: Data Not Collected.
>
> 4. Regions. The app works identically in all regions. The interface is localized into 11 languages (English, French, Spanish, German, Italian, Polish, Belarusian, Ukrainian, Simplified Chinese, Japanese, Korean) with no regional differences in features or content.
>
> 5. Regulated industry / third-party material. None. No in-app purchases, no user accounts, no user-generated content shared between users, no protected third-party material.
>
> Website and support: https://tabtool.shurco.app

Reply for the Resolution Center (post it with the resubmission):

> Hello, thank you for the review. I have added the requested information to the App Review Information Notes: the app's purpose and audience, setup instructions, the list of external services (none), regional behavior (identical in all regions), and confirmation that no regulated or protected third-party material is involved. A screen recording from a physical iPhone running the latest iOS, starting from app launch and showing the typical user flow, is attached. The app has no accounts, user-generated content, or paid features, so no login, account deletion, reporting, or purchase flows exist. Please let me know if anything else is needed.

Recording steps (iPhone, Control Center > Screen Recording): start the recording, open TabTool from the Home Screen, tap "+" > New Document, edit a few cells, add and delete a row and a column, sort a column, open Filters and Format, tap Export, open the overflow menu > Settings > About, stop the recording. Upload the video in App Store Connect with the submission.

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

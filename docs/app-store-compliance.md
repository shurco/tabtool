# App Store and privacy

This document describes what is needed to publish TabTool in the App Store. It does not replace the [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/) or account rules.

## What the app does

TabTool is a simple CSV editor for macOS. It opens CSV/TSV/text files, detects the format, shows a table, lets the user edit cells/rows/columns, and save or export the file in the selected encoding and with the selected delimiter. There are no formulas, macros, network, accounts, or purchases.

## Sandbox and file access

`app/apple/macOS/App/TabTool.entitlements`:

- `com.apple.security.app-sandbox` — mandatory sandbox.
- `com.apple.security.files.user-selected.read-write` — access only to files selected in open/save panels.

There is no network, iCloud, Apple Events, camera, microphone, contacts, or other permissions. The app does not download or execute code. `ENABLE_HARDENED_RUNTIME=YES` is set in `app/apple/project.yml`; the development archive is unsigned only because no team is configured yet.

## Build configuration

`app/apple/project.yml` is the source of truth:

- `INFOPLIST_FILE=macOS/App/Info.plist`;
- `CODE_SIGN_ENTITLEMENTS=macOS/App/TabTool.entitlements`;
- `ASSETCATALOG_COMPILER_APPICON_NAME=AppIcon`;
- `MACOSX_DEPLOYMENT_TARGET=14.0`, `SWIFT_VERSION=6.0`, `SWIFT_STRICT_CONCURRENCY=complete`;
- `ENABLE_TESTABILITY=YES` is Debug-only;
- `PrivacyInfo.xcprivacy` is copied into the app's resources.

`Info.plist` also carries `LSApplicationCategoryType=public.app-category.utilities`, `ITSAppUsesNonExemptEncryption=false`, `LSMinimumSystemVersion=$(MACOSX_DEPLOYMENT_TARGET)`, `NSPrincipalClass=NSApplication`, and `CFBundleDevelopmentRegion=en`. The required MIT notice is bundled from the repository `LICENSE` file.

## Privacy

- Data is not collected, transmitted, or used for tracking.
- No analytics, Crashlytics, Firebase, ads, or third-party SDKs.
- App Privacy in App Store Connect: **Data Not Collected**.
- `app/apple/macOS/App/PrivacyInfo.xcprivacy`: `NSPrivacyTracking=false`, empty `NSPrivacyCollectedDataTypes`, empty `NSPrivacyTrackingDomains`, and one accessed-API entry: `NSPrivacyAccessedAPICategoryUserDefaults` with reason `CA92.1`. The app stores only its own window and format-bar preferences with `@AppStorage`; this does not change the Data Not Collected answer. The file is copied into `Contents/Resources`.
- Local settings and unsaved data stay on the device; the app makes no network requests.
- `Shared/Localizable.xcstrings` contains the English UI strings; there are no hidden network or analytics strings.

## Document types

In `Info.plist`:

| Type | UTI | Role | Extensions |
|---|---|---|---|
| CSV | `public.comma-separated-values-text` | Editor, Owner | `.csv` |
| TSV | `public.tab-separated-values-text` | Editor, Owner | `.tsv` |
| Plain text | `public.plain-text` | Editor, Alternate | `.txt` |

Plain text is Alternate so the app does not intercept all text files. Arbitrary extensions are not opened.

## Name, bundle ID, icon

Before release, the build must have:

- a unique public name;
- a new bundle ID (in development `app.shurco.tabtool`);
- a signing team and provisioning setup;
- its own icon and branding. The repository ships a blue table placeholder in `Shared/Assets.xcassets/AppIcon.appiconset` (16–1024 px); replace it with final 1024 px artwork at release.

Choose a unique name and icon. The required MIT copyright and permission notice is kept in `LICENSE` and bundled with the app.

## Category and store card

- Category: Utilities.
- Subtitle and description: a simple CSV editor for macOS, without promises of formulas, databases, or "any tables".
- Screenshots: empty document, opened CSV, format panel, export.
- Keywords: CSV, TSV, editor, table, delimiter, encoding.
- The v1 language is English; the String Catalog is ready for Russian localization.

## Notes for Review

Briefly for the reviewer:

> TabTool is a local CSV/TSV/text editor. It reads a user-selected file, detects the delimiter/encoding/quote style, lets the user edit a grid, and saves or exports the result. It has no network access, no account, no in-app purchases, and collects no data. App Sandbox is enabled with user-selected read-write access only. Test files are included in the review notes/attachment.

If the reviewer needs a test file, attach a small CSV with commas, quotes, and a header; the format is detected automatically.

## Export encryption

The app does not implement its own cryptography and does not transmit data over the network. `ITSAppUsesNonExemptEncryption=false`; the account owner confirms the exemption basis.

## Before submitting

- [ ] Public name, bundle ID, signing team are chosen; `app.shurco.tabtool` is gone from `project.yml` and `Info.plist`.
- [ ] Final icon replaces the placeholder in `Shared/Assets.xcassets/AppIcon.appiconset`.
- [ ] `version.json` carries the release version and build.
- [ ] `./scripts/test` and `./scripts/test --ui` pass.
- [ ] `./scripts/app archive macos` succeeds; inspect `Info.plist`, entitlements, `PrivacyInfo.xcprivacy`, and `AppIcon.icns` in the archive.
- [ ] The archive is signed with the team and validated in Xcode Organizer or Transporter.
- [ ] App Privacy: Data Not Collected.
- [ ] Screenshots and description without promises of formulas/databases.
- [ ] The required MIT notice is bundled with the app and kept in `LICENSE`.
- [ ] Sandbox, open/save, export, errors, dark mode, VoiceOver, and a 100k-row file verified on a clean system.
- [ ] `version.lock.json` updated after upload.

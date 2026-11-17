# Android build, signing and release

The Android app is `app/universal/apps/tabtool-android`: the Rust `cdylib` plus
the Gradle host described in
[Universal (Rust)](../app/universal/docs/universal.md#android-host). This page
covers packaging: version, icons, debug and release builds, signing, what the
artifacts contain, and the CI job. The Apple app in `app/apple` stays the source
of truth for behavior and for the icon artwork.

## Toolchain

| Tool | Version |
| --- | --- |
| Rust | 1.99.1 with `aarch64-linux-android` and `x86_64-linux-android` |
| `cargo-ndk` | 4.1.2 |
| Gradle | 8.14.7 (wrapper committed) |
| Android Gradle Plugin | 8.13.2 |
| Kotlin Gradle plugin | 2.4.21 |
| JDK | 17–21 (Gradle 8.x cannot run on the bundled JBR 25) |
| SDK | compileSdk 36, targetSdk 36, minSdk 26 |
| NDK | 28.2.13676358 |
| ABIs | `arm64-v8a`, `x86_64` |

The app declares no `INTERNET` permission: the only permission in the merged
manifest is
`app.shurco.tabtool.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION`, which
`androidx.core` defines for its own unprotected receivers. Files arrive through
the activity's `VIEW` and `SEND` intent filters and through storage access
framework pickers, never over the network.

## Build

```sh
export JAVA_HOME="/opt/homebrew/opt/openjdk@17"      # any JDK 17-21
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"

cd app/universal/apps/tabtool-android/android

# Native libraries first: Gradle packages whatever is in jniLibs. cargo ndk
# only copies an artifact newer than the file already there, so remove the
# directory when switching variant.
rm -rf app/src/main/jniLibs
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build -p tabtool-android          # debug
./gradlew assembleDebug           # app/build/outputs/apk/debug/app-debug.apk

rm -rf app/src/main/jniLibs
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build --release -p tabtool-android # release
./gradlew assembleRelease         # app/build/outputs/apk/release/app-release.apk
./gradlew bundleRelease           # app/build/outputs/bundle/release/app-release.aab
```

`app/src/main/jniLibs` and every APK and AAB are gitignored: the libraries are
built, never committed. Gradle packages the `libmain.so` that `cargo ndk`
wrote, and the `verifyNativeLibraries` task that every build depends on fails
when a library is missing, older than the newest Rust source, or not
byte-identical to the `target/<triple>/<profile>/libmain.so` cargo built for the
requested variant. The byte check matters because `cargo ndk` prints "Copying
libraries" even when it leaves the other variant's copy in place.

One `jniLibs` directory cannot serve both variants, so pair the `cargo ndk` run
with the matching Gradle task: `cargo ndk … build` with `assembleDebug`, or
`cargo ndk … build --release` with `assembleRelease bundleRelease`. `cargo ndk`
only copies an artifact that is newer than the file already in
`src/main/jniLibs`, so remove the directory first when you switch variant.
`./gradlew assemble` and `./gradlew build` package both variants from that one
directory and therefore fail with an explanation instead of shipping the wrong
libraries.

### Packaging

- R8 and resource shrinking are off for the release build.
- `packaging { jniLibs { useLegacyPackaging = false } }` states the Android
  Gradle Plugin default: the native libraries are stored uncompressed in the
  APK, the merged manifest carries `android:extractNativeLibs="false"`, and
  the app loads `libmain.so` from the APK in place instead of extracting it at
  install time. `scripts/android-verify` checks the stored entries and the
  `zipalign -c -P 16` page alignment, which Android 15 devices with 16 KB pages
  need.
- Both ABIs are unpacked from the same `jniLibs` directory, so one `cargo ndk`
  run produces both the APK and the AAB contents.

## Install and run

```sh
adb install -r app/build/outputs/apk/release/app-release.apk
adb shell am start -n app.shurco.tabtool/.MainActivity
```

The debug build has the same behavior with `app-debug.apk`. `./gradlew
installDebug` does the install step when exactly one device is attached.

## Intents and file handoff

`MainActivity` is `singleTask` and exported. It declares four intent filters:

- the launcher;
- `ACTION_VIEW` for `text/csv`, `text/comma-separated-values`,
  `application/csv`, `text/tab-separated-values` and `text/plain`;
- `ACTION_VIEW` for `file://` URIs ending in `.csv`, `.tsv`, `.txt`, `.tab` or
  `.dat`, the form older file managers send;
- `ACTION_SEND` with the same table MIME types.

A `VIEW` or `SEND` stream is copied, not read into memory: Kotlin writes it to a
hidden `.incoming-*` file inside the private documents directory and reports the
staging name and the display name to Rust. `apps/tabtool-android/src/intents.rs`
then:

- reduces the display name to one safe segment (control characters and path
  separators removed, a leading dot stripped, the stem capped at 100
  characters);
- keeps an accepted table extension and otherwise uses the staging file's
  extension;
- picks a free name case-insensitively, so `data.csv` becomes `data 2.csv`
  instead of replacing a document;
- removes the staging file on every failure and reports a localized banner.

Two quick intents are both adopted; the last one opens. The intent that was
already consumed is remembered in the saved instance state, so a recreation
does not import it twice. Startup removes abandoned staging files older than an
hour. The share path is the same private file through `FileProvider`
(`app.shurco.tabtool.fileprovider`, paths XML, `exported="false"`).

`adb` checks:

```sh
adb shell am start -a android.intent.action.VIEW -d content://... -t text/csv
adb shell am start -a android.intent.action.SEND -t text/csv \
  --eu android.intent.extra.STREAM content://...
adb shell "run-as app.shurco.tabtool ls files/documents"
```

## Storage access framework

The app declares no storage permission and does not use `WRITE_EXTERNAL_STORAGE`
or `MANAGE_EXTERNAL_STORAGE`.

- Import uses `ACTION_OPEN_DOCUMENT` with `CATEGORY_OPENABLE` and `text/*`.
  Kotlin streams the chosen `content://` URI into the private documents
  directory, deletes the copy when its name or extension is not listable, and
  returns the private path.
- Export writes a converted temporary CSV, then `ACTION_CREATE_DOCUMENT` streams
  that file to the user's location. A failed or cancelled export leaves the
  working document untouched.
- Save As for a large document copies the mapped file through the same SAF
  create flow. The editor keeps its private working file and shows the
  localized “Saved a copy to …” notice; the copy is not reopened.
- The pickers answer `null` while open and report their result on a later poll,
  so the UI never blocks the frame.

## Lifecycle, autosave and process death

Rotating, folding, resizing and showing the keyboard do not recreate the
activity: the manifest lists `density`, `keyboard`, `keyboardHidden`,
`navigation`, `orientation`, `screenLayout`, `screenSize`, `smallestScreenSize`
and `uiMode` in `configChanges`. The app re-lays out from the new size, safe
area and keyboard inset, and keeps the document, selection and open native edit.

- Small file-backed documents autosave 0.8 s after the last change
  (`tabtool-document`'s `AUTOSAVE_DELAY`). Untitled, large and write-blocked
  documents never autosave.
- Sending the app to the background requests a save from the pause path. Rust
  acknowledges within 400 ms so the activity does not hang; a large document is
  left dirty because only the explicit streaming save may write it.
- Back closes one sheet at a time, then prompts when the document is dirty and
  cannot be saved safely (large or write-blocked), then closes the editor, and
  only exits at the library.
- The only process-death state is `files/app-state.json`:
  `{"version":1,"document":"…","sheet":"settings","sheets":["settings"]}`. It is
  written atomically through a `.json.part` file. A missing or malformed file
  falls back to the library. Unsaved edits are not part of it; the last save on
  disk wins.
- The system theme is read from the activity's night bit on every frame, because
  winit reports no theme on Android. Light mode also sets dark status and
  navigation bar icons.

## Known deviations

- Android always uses the mobile layout. There is no desktop layout, menu bar,
  multiple windows or desktop format/filter bars on Android.
- Save As is a copy through SAF; the editor stays on its private working file.
  There is no “save in place to another location”.
- There is no review prompt. The About sheet's Rate link opens the project
  website until a store listing exists.
- Fonts come from the system. A device without a CJK or emoji font shows the
  fallback glyph for that character; no font is bundled.
- The app has no network permission, no account, no cloud sync and no store
  metadata in this repository.
- The fold-aware action bar is implemented and unit-tested, but the platform
  host has no foldable AVD in CI or on this machine. Hinge placement is covered
  by `tests/mobile/fold.rs` and the folded-size snapshots; a physical foldable
  device is still required to confirm it.

## Verification

The mobile suites are cross-platform and need no device:

```sh
cd app/universal
cargo test -p tabtool-ui --test mobile          # scenarios, touch, sheets, a11y
cargo test -p tabtool-android --test android_app # lifecycle, intents, autosave
TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 \
  cargo test -p tabtool-ui --test mobile visual:: # local image baselines
```

The image baselines cover the phone (360×800), tablet (800×1280), a closed
foldable (411×797) and the open inner display (841×701) with an active hinge.
They are per-machine and not committed. The manual emulator pass, with its
PASS/FAIL/NOT VERIFIED evidence, is in
[android-checklist.md](android-checklist.md).

## Version

The Android version is the `universal` entry of `app/version.json` (the same
`name` is the desktop version, see [release.md](release.md)). The Apple store
versions sit in the same file as separate `macos` and `ios` entries because the
two stores version independently:

```json
{
  "universal": { "name": "1.0.0", "build": "1" }
}
```

`name` is the Gradle `versionName` and `build` is the `versionCode`, so the name
is visible in the app info screen and the build number must increase for every
upload. The only writer of `universal` is `xtask`; it keeps the other entries
intact:

```sh
cd app/universal
cargo run -p xtask -- version                 # Version (universal): 1.0.0 (1)
cargo run -p xtask -- version bump            # build only
cargo run -p xtask -- version bump patch
cargo run -p xtask -- version bump minor
cargo run -p xtask -- version bump major
```

Bumping follows `scripts/lib/version`, the Apple script's version library: the
build number always increases, and patch, minor and major also move the
semantic version.

| Kind | `name` | `build` |
| --- | --- | --- |
| `build` (default) | unchanged | +1 |
| `patch` | patch +1 | +1 |
| `minor` | minor +1, patch 0 | +1 |
| `major` | major +1, minor and patch 0 | +1 |

The bump validates the file first and rewrites it through a temporary file, so
an interrupted bump cannot corrupt it. `app/universal/apps/tabtool-android/android/app/build.gradle.kts`
reads the same file and rejects a missing file, a name that is not `x.y.z`, and
a build that is not a positive integer, so a malformed version stops the build
instead of shipping.

## Icons

The icon has one source, the Apple 1024 px artwork in
`app/apple/Shared/Assets.xcassets/AppIcon.appiconset/icon_ios_1024.png`. The
Android resources are generated from it and committed:

```sh
./scripts/android-icons
```

The script needs `sips`, so it runs on macOS, where the Apple artwork also
lives. It writes:

- `mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher.png` — 48, 72, 96, 144
  and 192 px, the legacy icon for launchers that ignore adaptive icons;
- `mipmap-*/ic_launcher_foreground.png` — 108, 162, 216, 324 and 432 px, the
  adaptive foreground;
- `mipmap-anydpi-v26/ic_launcher.xml` — the adaptive icon: an
  `@color/ic_launcher_background` layer and the foreground inset by 19.4% per
  side. The artwork is then 66 dp of the 108 dp canvas, so the glyph (63% of
  the artwork) is about 42 dp and its circumscribed circle (59 dp) stays inside
  the 66 dp adaptive safe zone: a circular or squircle mask never cuts the
  frame;
- `values/ic_launcher_background.xml` — `#121310`, the background of the Apple
  artwork.

The manifest points `android:icon` at `@mipmap/ic_launcher`. Rerun the script
only when the Apple icon changes, and commit the regenerated resources with
it. There is no monochrome layer, so themed icons use the colored icon.

## Signing

Debug builds use the standard Android debug keystore, which Gradle creates in
`~/.android/debug.keystore` on first use; no key material is committed.

Release builds read four environment variables:

| Variable | Value |
| --- | --- |
| `TABTOOL_ANDROID_KEYSTORE` | path to the keystore (`.jks` or `.keystore`) |
| `TABTOOL_ANDROID_KEYSTORE_PASSWORD` | store password |
| `TABTOOL_ANDROID_KEY_ALIAS` | key alias |
| `TABTOOL_ANDROID_KEY_PASSWORD` | key password |

```sh
keytool -genkeypair -v -keystore ~/tabtool-android.jks -alias tabtool \
  -keyalg RSA -keysize 4096 -validity 10000

export TABTOOL_ANDROID_KEYSTORE="$HOME/tabtool-android.jks"
export TABTOOL_ANDROID_KEYSTORE_PASSWORD="…"
export TABTOOL_ANDROID_KEY_ALIAS="tabtool"
export TABTOOL_ANDROID_KEY_PASSWORD="…"
./gradlew --console=plain assembleRelease bundleRelease
```

With all four set, the release APK and AAB are signed with that key. Without
them the release build is signed with the debug keystore and Gradle prints:

```
TabTool: release signing is not configured. Set TABTOOL_ANDROID_KEYSTORE, …; this build is debug-signed.
```

That fallback keeps a local release build installable and verifiable, and it
must never be uploaded. Key material is never committed: `.gitignore` excludes
`*.jks`, `*.keystore`, `*.p12` and `*.pem`.

## Verifying an artifact

`scripts/android-verify` checks the packaging contract and exits non-zero on
the first violation. It needs `ANDROID_HOME` (or `ANDROID_SDK_ROOT`), python3
and `JAVA_HOME` for the signature check; it uses the newest installed
build-tools:

```sh
cd <repository root>
scripts/android-verify app/universal/apps/tabtool-android/android/app/build/outputs/apk/debug/app-debug.apk
scripts/android-verify --release app/universal/apps/tabtool-android/android/app/build/outputs/apk/release/app-release.apk
```

It always checks the version against `universal` in `app/version.json`, both native
ABIs, the absence of the `INTERNET` permission, the stored native entries and
the `zipalign -c -P 16` page alignment. `--release` additionally requires a
verifying signature that is not `CN=Android Debug`, so a release build that
fell back to the debug keystore fails instead of passing.

The individual commands, with the same newest-build-tools lookup:

```sh
AAPT2="$(ls -d "$ANDROID_HOME"/build-tools/*/aapt2 | sort -V | tail -1)"
APKSIGNER="$(ls -d "$ANDROID_HOME"/build-tools/*/apksigner | sort -V | tail -1)"
ZIPALIGN="$(ls -d "$ANDROID_HOME"/build-tools/*/zipalign | sort -V | tail -1)"

"$AAPT2" dump badging app/build/outputs/apk/release/app-release.apk | \
  grep -E "^package|sdkVersion|native-code|launchable-activity"
"$AAPT2" dump permissions app/build/outputs/apk/release/app-release.apk
unzip -v app/build/outputs/apk/release/app-release.apk | grep 'lib/'
"$ZIPALIGN" -c -P 16 -v 4 app/build/outputs/apk/release/app-release.apk
"$APKSIGNER" verify --print-certs app/build/outputs/apk/release/app-release.apk
```

What a correct release artifact shows:

- `package: name='app.shurco.tabtool' versionCode='1' versionName='1.0.0'` — the
  values of `universal` in `app/version.json`;
- `minSdkVersion:'26'` and `targetSdkVersion:'36'`;
- `native-code: 'arm64-v8a' 'x86_64'`, with `lib/arm64-v8a/libmain.so` and
  `lib/x86_64/libmain.so` inside the APK, both `Stored` (uncompressed) and
  `(OK)` in `zipalign -c -P 16`;
- no `uses-permission: name='android.permission.INTERNET'`;
- the four intent filters on `MainActivity` (launcher, `VIEW` for CSV and TSV
  MIME types, `VIEW` for `file://` table extensions, `SEND`), all exported
  because they are the entry points, and `FileProvider` with
  `exported="false"`;
- `apksigner verify` reporting the release certificate; `CN=Android Debug`
  means the build fell back to the debug keystore and is not a release.

The AAB carries the same libraries under `base/lib/<abi>/libmain.so`. The
launcher icon is verified once by installing the APK and looking at the app
drawer: the icon must show the white grid frame and the blue center cell inside
whatever mask the launcher uses.

## CI

`.github/workflows/android.yml` is a reusable workflow. `universal.yml` calls it
on every push and pull request touching `app/universal`, and `release.yml`
calls it for tags and manual builds ([release.md](release.md)):

- JDK 17, Rust 1.99.1 with both Android targets, and the NDK 28.2.13676358,
  installed only when the runner does not already have it;
- the Cargo registry, `~/.cargo/bin` and `app/universal/target` cached by
  `actions/cache`, and the Gradle user home and build cache by
  `gradle/actions/setup-gradle`;
- `cargo-ndk` 4.1.2 from crates.io, reinstalled when the cached binary is not
  that version;
- `cargo ndk` for both ABIs, `./gradlew assembleDebug`, then
  `scripts/android-verify` on the debug APK (version, ABIs, no network, stored
  libraries, 16 KB alignment);
- the debug APK is uploaded as the `tabtool-android-debug` artifact.

The release APK and AAB are built only when the repository has the signing
secrets. They are named `TABTOOL_ANDROID_KEYSTORE_BASE64` (the keystore,
base64-encoded, because a secret cannot hold a file), plus
`TABTOOL_ANDROID_KEYSTORE_PASSWORD`, `TABTOOL_ANDROID_KEY_ALIAS` and
`TABTOOL_ANDROID_KEY_PASSWORD`. A tag build (`require-signing: true`) fails
without them. The job also fails when the base64 keystore is set
but any of the other three is empty, so a half-configured release cannot fall
back to the debug key. It decodes the keystore into `$RUNNER_TEMP`, points
`TABTOOL_ANDROID_KEYSTORE` at it, builds both artifacts, runs
`scripts/android-verify --release` on the APK and checks both
`base/lib/<abi>/libmain.so` entries in the AAB before uploading
`tabtool-android-release`. Without the secrets the job still builds and checks
the debug APK; no emulator runs in CI.

## Limits

The app is one activity with no background services, no network permission and
no store metadata in this repository. `minSdk` 26 excludes Android 7.x and
older, and the x86_64 library exists for emulators only. Nothing here uploads a
signed AAB to Google Play: a tag attaches the signed APK and AAB to the GitHub
Release, and the upload to Play stays manual.

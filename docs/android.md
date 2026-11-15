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

# Native libraries first: Gradle packages whatever is in jniLibs.
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build -p tabtool-android          # debug
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build --release -p tabtool-android # release

./gradlew assembleDebug           # app/build/outputs/apk/debug/app-debug.apk
./gradlew assembleRelease         # app/build/outputs/apk/release/app-release.apk
./gradlew bundleRelease           # app/build/outputs/bundle/release/app-release.aab
```

`app/src/main/jniLibs` and every APK and AAB are gitignored: the libraries are
built, never committed. Gradle packages the `libmain.so` that `cargo ndk`
wrote, so a Gradle-only build on a clean checkout produces an APK that installs
and then fails to load `GameActivity`; run `cargo ndk` first. The directory
holds whichever variant ran last, so rerun `cargo ndk` with or without
`--release` to match the APK you are packaging: a debug build after a release
`cargo ndk` ships release libraries.

R8 and resource shrinking are off for the release build. Both build types leave
the native libraries uncompressed in the APK (the Android Gradle Plugin
default), so they are loaded in place instead of being copied at install time.

## Install and run

```sh
adb install -r app/build/outputs/apk/release/app-release.apk
adb shell am start -n app.shurco.tabtool/.MainActivity
```

The debug build has the same behavior with `app-debug.apk`. `./gradlew
installDebug` does the install step when exactly one device is attached.

## Version

The Android version lives in `app/universal/version.json`, apart from the Apple
store version in `app/apple/version.json` because the two stores version
independently:

```json
{
  "name": "1.0.0",
  "build": "1"
}
```

`name` is the Gradle `versionName` and `build` is the `versionCode`, so the name
is visible in the app info screen and the build number must increase for every
upload. The only writer is `xtask`:

```sh
cd app/universal
cargo run -p xtask -- version                 # Version (android): 1.0.0 (1)
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
  `@color/ic_launcher_background` layer and the foreground inset by 18.6%, the
  frame inset of the Apple artwork, so a circular or squircle launcher mask
  keeps the frame instead of cutting its corners;
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

```sh
AAPT2="$ANDROID_HOME/build-tools/36.1.0/aapt2"

"$AAPT2" dump badging app/build/outputs/apk/release/app-release.apk | \
  grep -E "^package|sdkVersion|native-code|launchable-activity"
"$AAPT2" dump permissions app/build/outputs/apk/release/app-release.apk
unzip -l app/build/outputs/apk/release/app-release.apk | grep '^ *[0-9]*.*lib/'
"$ANDROID_HOME/build-tools/36.1.0/apksigner" verify --print-certs \
  app/build/outputs/apk/release/app-release.apk
```

What a correct release artifact shows:

- `package: name='app.shurco.tabtool' versionCode='1' versionName='1.0.0'` — the
  values in `app/universal/version.json`;
- `minSdkVersion:'26'` and `targetSdkVersion:'36'`;
- `native-code: 'arm64-v8a' 'x86_64'`, with `lib/arm64-v8a/libmain.so` and
  `lib/x86_64/libmain.so` inside the APK;
- no `uses-permission: name='android.permission.INTERNET'`;
- the four intent filters on `MainActivity` (launcher, `VIEW` for CSV and TSV
  MIME types, `VIEW` for `file://` table extensions, `SEND`), all exported
  because they are the entry points, and `FileProvider` with
  `exported="false"`;
- `apksigner verify` reporting the release certificate, or `CN=Android Debug`
  when the build fell back to the debug keystore.

The AAB carries the same libraries under `base/lib/<abi>/libmain.so`. The
launcher icon is verified once by installing the APK and looking at the app
drawer: the icon must show the white grid frame and the blue center cell inside
whatever mask the launcher uses.

## CI

`.github/workflows/universal.yml` has an `android` job that runs on every push
and pull request touching `app/universal`:

- JDK 17, Rust 1.99.1 with both Android targets, and the NDK 28.2.13676358,
  installed only when the runner does not already have it;
- `cargo-ndk` 4.1.2 from crates.io, cached in `~/.cargo/bin` with the Cargo
  registry, `app/universal/target` and the Gradle cache;
- `cargo ndk` for both ABIs, `./gradlew assembleDebug`, and a check of the
  built APK's version, ABIs and permissions;
- the debug APK is uploaded as the `tabtool-android-debug` artifact.

The release APK and AAB are built only when the repository has the signing
secrets. They are named `TABTOOL_ANDROID_KEYSTORE_BASE64` (the keystore,
base64-encoded, because a secret cannot hold a file), plus
`TABTOOL_ANDROID_KEYSTORE_PASSWORD`, `TABTOOL_ANDROID_KEY_ALIAS` and
`TABTOOL_ANDROID_KEY_PASSWORD`. The job decodes the keystore into
`$RUNNER_TEMP`, points `TABTOOL_ANDROID_KEYSTORE` at it, and uploads
`tabtool-android-release`. Without the secrets the job still builds and checks
the debug APK; no emulator runs in CI.

## Limits

The app is one activity with no background services, no network permission and
no store metadata in this repository. `minSdk` 26 excludes Android 7.x and
older, and the x86_64 library exists for emulators only. Nothing here uploads a
signed AAB to Google Play: the CI job stops at the artifacts.

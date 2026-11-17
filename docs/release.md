# Releases of the Rust apps

`.github/workflows/release.yml` builds the Windows, Linux and Android apps from
`app/universal`. The Apple apps are not part of it; they ship through Xcode
(see [building.md](building.md)).

## Version

The `universal` entry of `app/version.json` is the single version source. Gradle
reads its `name` and `build` for `versionName` and `versionCode`, and the
`tabtool-core` build script exposes its `name` as `tabtool_core::VERSION`, which
the desktop and Android About screens and `tabtool-desktop --smoke` print. Bump
it with `cargo run -p xtask -- version bump <build|patch|minor|major>` before
tagging; `xtask` leaves the Apple `macos` and `ios` entries untouched.

## Tag release

Pushing a tag `v<name>` (for example `v1.0.0`) runs every platform and
publishes a GitHub Release:

```sh
git tag v1.0.0
git push origin v1.0.0
```

- The `plan` job fails when the tag is not `v` plus `universal.name` in
  `app/version.json`, so a tag can never ship a different version.
- The Android build must be signed: without the four
  `TABTOOL_ANDROID_KEYSTORE*` / `TABTOOL_ANDROID_KEY_*` secrets (see
  [android.md](android.md#ci)) the tag run fails instead of publishing a debug
  APK.
- The Release is created only after every build succeeds. Its notes are
  generated from the commits since the previous tag.

Release files:

| File | Contents |
|---|---|
| `tabtool-<version>-windows-x86_64.zip` | `tabtool.exe`, `LICENSE` |
| `tabtool-<version>-windows-aarch64.zip` | `tabtool.exe`, `LICENSE` |
| `tabtool-<version>-linux-x86_64.tar.gz` | `tabtool`, `LICENSE` |
| `tabtool-<version>-linux-aarch64.tar.gz` | `tabtool`, `LICENSE` |
| `tabtool-<version>-android.apk` | signed release APK (arm64-v8a, x86_64) |
| `tabtool-<version>-android.aab` | signed bundle for Google Play |
| `SHA256SUMS` | SHA-256 of every file above |

## Manual build

Actions > Release > Run workflow builds the platforms ticked in the form
(Windows, Linux, Android; all on by default) from the selected branch. The
files stay as run artifacts and no Release is created. The version is
`<name>-<short commit>`, for example `1.0.0-9a5a428`. Android always produces
the debug APK and adds the signed APK and AAB when the secrets exist.

From the command line:

```sh
gh workflow run release.yml --ref <branch> -f windows=true -f linux=false -f android=true
```

GitHub shows the Run workflow button and accepts `gh workflow run` only once
`release.yml` is on the default branch.

## Build hosts

Every desktop target builds natively, so the smoke run checks the binary that
ships:

| Target | Runner |
|---|---|
| `x86_64-pc-windows-msvc` | `windows-2022` |
| `aarch64-pc-windows-msvc` | `windows-11-arm` |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` |

The arm runners are free for public repositories. The Linux binaries need
glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36) and fontconfig; X11
or Wayland and xkbcommon are loaded at run time. The archives are unsigned: no
Authenticode signature on Windows, no installer and no `.desktop` entry on
Linux.

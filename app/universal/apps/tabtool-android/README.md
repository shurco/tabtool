# TabTool Android host (feasibility spike)

Real Android host for the Rust `tabtool-core` port: a Gradle project with a
`GameActivity` subclass, Kotlin JNI bridge, and a Rust `cdylib` built with eframe
0.37.0 / egui 0.37.0.

This is a developer spike, not product UI. Its purpose is to decide whether the
Android input path (soft keyboard + IME composition), clipboard, Storage Access
Framework (SAF) and window insets support the port. The checklist below is the
go/no-go record.

## Toolchain

| Tool | Version |
| --- | --- |
| Rust | 1.99.1 (rustup, targets `aarch64-linux-android`, `x86_64-linux-android`) |
| cargo-ndk | 4.1.2 |
| Gradle | 8.14.7 (wrapper committed) |
| Android Gradle Plugin | 8.13.2 |
| Kotlin Gradle plugin | 2.4.21 |
| `androidx.games:games-activity` | 4.4.2 |
| `androidx.appcompat:appcompat` | 1.8.0 |
| `androidx.core:core-ktx` | 1.18.0 |
| NDK | 28.2.13676358 |
| compileSdk / targetSdk / minSdk | 36 / 36 / 26 |
| JDK | Temurin 21.0.13 (Gradle 8.x cannot run on the bundled JBR 25) |
| Emulator | `Medium_Phone_API_36.1` (arm64, Android 16), headless |

`androidx.core` 1.19.x requires compileSdk 37 and AGP 9.1, so 1.18.0 is the
newest version compatible with the mandated compileSdk 36 / Gradle 8.x.

## Build

Set the SDK, NDK and JDK first:

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export JAVA_HOME="$HOME/.local/jdks/jdk-21.0.13+9/Contents/Home"   # any JDK 17-21

cd app/universal/apps/tabtool-android/android

# Build the Rust libraries into app/src/main/jniLibs (gitignored).
cargo ndk -t arm64-v8a -t x86_64 -o app/src/main/jniLibs build -p tabtool-android

./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n app.shurco.tabtool/.MainActivity
```

The crate's library name is `main` because `GameActivity` loads `libmain.so`.

## Bridge surface

`app.shurco.tabtool.TabToolBridge` (Kotlin) exposes static JNI methods consumed
by `src/platform_bridge.rs`:

| Kotlin | Rust | Notes |
| --- | --- | --- |
| `clipboardGet(): String?` | `clipboard_get() -> Option<String>` | `SecurityException` is caught in Kotlin and cleared in Rust |
| `clipboardSet(text: String)` | `clipboard_set(&str)` | |
| `openDocument(): String?` | `open_document() -> Option<String>` | launches `ACTION_OPEN_DOCUMENT` (`text/*`) with `ACTION_GET_CONTENT` fallback; returns and clears the URI from the previous pick |
| `insets(): IntArray` | `insets() -> Insets` | `[left, top, right, bottom, keyboard]` in dp |
| `throwSecurityException()` | spike self-test | proves JNI exceptions are cleared instead of crashing |

## Checklist result

Verified on the `Medium_Phone_API_36.1` AVD (arm64, Android 16, headless,
`-gpu host`, Gboard). Screenshots were taken with `adb exec-out screencap`.

| # | Check | Result |
| --- | --- | --- |
| 1 | Soft keyboard opens on tap; Latin text appears and commits | **FAIL** |
| 2 | Russian IME: Cyrillic `Привет` commits | **FAIL (blocked by 1)** |
| 3 | Chinese Pinyin composition and commit (no duplication/loss) | **NOT VERIFIED** |
| 4 | Clipboard set/get transfers text with another app | **PASS** |
| 5 | `Open document` returns a `content://` URI for a picked CSV | **PASS** |
| 6 | Insets change with keyboard and rotation; values sane | **PASS** |
| 7 | Back gesture and rotation do not crash or lose typed text | **PARTIAL / NOT VERIFIED** |

### Evidence

1. Tapping the field opens Gboard and `insets().keyboard` goes from 0 to 336 dp,
   so `show_soft_input` works. However, neither tapping Gboard keys nor
   `adb shell input text` / `input keyevent` changes the field or the
   `value:` label. Gboard accumulates the composition in its suggestion strip
   (`Hello`, `Helloabc`) but never commits it to the app. The cause is in the
   stack, not the host code: `android-activity` 0.6.1 delivers soft-keyboard
   input as `android_activity::input::InputEvent::TextEvent`, and winit 0.30.13
   only handles `InputEvent::KeyEvent` and `InputEvent::MotionEvent` in
   `src/platform_impl/android/mod.rs`; `TextEvent` falls into the
   `Unknown android_activity input event` arm. eframe 0.37.0 depends on
   winit 0.30.13, so IME commits are dropped before egui sees them.
2. Blocked by check 1: no IME text reaches the app, so Cyrillic cannot be
   exercised. The failure is independent of the keyboard language.
3. A Pinyin layout was not configured on the AVD (Gboard only had Latin), and
   the missing `TextEvent` path blocks composition for every IME. Not marked
   FAIL only because the IME was unavailable; the base defect is recorded in 1.
4. `Clipboard set` → Chrome paste produced "TabTool clipboard" and the system
   toast "Chrome pasted from your clipboard". Copying in Chrome and pressing
   `Clipboard get` in the spike produced
   `Some("TabTool clipboardReverseClipTest")`. The forced
   `throwSecurityException` self-test showed `SecurityException contained:
   true` and the process stayed alive, so the JNI exception guard works.
5. `Open document` opened DocumentsUI filtered to `text/*`; picking
   `/sdcard/Download/sample.csv` yielded
   `content://com.android.providers.downloads.documents/document/raw%3A%2Fstorage%2Femulated%2F0%2FDownload%2Fsample.csv`.
6. Keyboard: `keyboard` 0 → 336 dp and back to 0. Gesture navigation portrait:
   `top 24, bottom 24`; 3-button landscape: `top 24, right 48`; 3-button
   portrait: `top 24, bottom 48`. Rotation re-laid out the egui surface without
   recreating the activity.
7. Rotation portrait↔landscape did not crash and the app remained resumed; a
   back key press did not crash or finish the activity. The "typed text is not
   lost" half cannot be observed because no text can be entered (check 1).

## Root cause and next step

`eframe` 0.37.0 / `egui-winit` 0.37.0 / winit 0.30.13 do not map Android
GameActivity `TextEvent` (GameTextInput's InputConnection state) into
`egui::Event::Ime`. Android soft-keyboard text is therefore not usable with
this exact stack. Options for a re-decision, in order of preference:

1. Patch/vendor winit's Android backend to translate `InputEvent::TextEvent`
   into `WindowEvent::Ime` preedit/commit events (the state carries the full
   text plus composing region, so a diff against the previous state is needed).
2. Add a Kotlin `EditText`/`InputConnection` overlay that forwards text to Rust
   through a dedicated JNI bridge, bypassing winit.
3. Re-evaluate the UI toolkit for Android if neither patch is acceptable.

The clipboard, SAF and insets bridges are proven and can be reused.


## Notes

- Last stable `androidx.core` (1.19.x) and AGP 9.x require compileSdk 37, so the
  Gradle 8.x / compileSdk 36 constraint pins AGP 8.13.2 and core-ktx 1.18.0.
- `games-activity` 4.4.2 extends `AppCompatActivity`, so the manifest uses
  `Theme.AppCompat.NoActionBar` and the project depends on appcompat.
- The AVD's Vulkan driver crashes inside wgpu adapter setup, so the app pins
  wgpu to its GL backend on Android and the emulator is started with
  `-gpu host`. Disabling guest Vulkan (`-feature -Vulkan`) instead made GL
  surface creation fail with `BadAlloc`.

# TabTool Android host (feasibility spike)

Real Android host for the Rust `tabtool-core` port: a Gradle project with a
`GameActivity` subclass, Kotlin JNI bridge, native `EditText` overlay, and a Rust
`cdylib` built with eframe 0.37.0 / egui 0.37.0.

This is a developer spike, not product UI. Its purpose is to decide whether the
Android input path (soft keyboard + IME composition), clipboard, Storage Access
Framework (SAF) and window insets support the port. The checklist below is the
go/no-go record.

## Why text entry uses a native editor

winit 0.30.13's Android backend handles only `InputEvent::KeyEvent` and
`InputEvent::MotionEvent`; GameActivity's soft-keyboard commits arrive as
`android_activity::input::InputEvent::TextEvent` and are dropped before
`egui-winit`. A stock egui `TextEdit` therefore never receives IME text on this
stack (verified in the first spike run).

The design keeps egui for everything else and opens a real Android
`AppCompatEditText` above the egui surface for text entry. The editor reports
`Committed(text)` or `Cancelled` through JNI callbacks; Rust drains the outcome
with `text_input::poll()`. winit is not forked or patched.

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
| `showTextEditor(left, top, width, height, text, multiline)` | `text_input::open(TextInputRequest)` | rectangle in physical pixels; opens the native `EditText` |
| `hideTextEditor()` | `text_input::close()` | removes the editor without committing or cancelling |
| `throwSecurityException()` | spike self-test | proves JNI exceptions are cleared instead of crashing |

`TabToolTextEditor` (real `AppCompatEditText`) calls back into Rust via
`nativeTextCommitted(text)` / `nativeTextCancelled()`. The callbacks store the
outcome in a `Mutex<Option<TextInputOutcome>>` that `text_input::poll()` drains.
Back cancels; the IME Done action and focus loss commit; rotation cancels the
edit without touching the committed value.

## Checklist result

Verified on the `Medium_Phone_API_36.1` AVD (arm64, Android 16, headless,
`-gpu host`, Gboard with English, Russian and Simplified Chinese/Pinyin
layouts). Screenshots were taken with `adb exec-out screencap`.

| # | Check | Result |
| --- | --- | --- |
| 1 | Tapping `Edit text` opens the native editor with the current text and soft keyboard; Latin text commits on Done | **PASS** |
| 2 | Russian IME: Cyrillic `Привет` commits | **PASS** |
| 3 | Chinese Pinyin composition and commit, no duplication/loss | **PASS** (payload verified; see font note) |
| 4 | Clipboard set/get transfers text with another app | **PASS** |
| 5 | `Open document` returns a `content://` URI for a picked CSV | **PASS** |
| 6 | Insets change with keyboard and rotation; values sane | **PASS** |
| 7 | Back gesture and rotation do not crash; Back closes the editor without changing text; rotation loses no committed value | **PASS** |

### Evidence

1. `Edit text` opened the native editor exactly over the egui placeholder with
   the current text and Gboard; `keyboard` inset went 0 → 336 dp. Typing
   `Hello` and pressing the IME Done action closed the editor and the Rust
   label showed `"Hello"` once.
2. Added the Russian layout in Gboard settings (Languages → Add keyboard →
   Русский), switched with the language key, typed `Привет` on the ЙЦУКЕН
   keyboard and pressed Done. Rust label: `"Привет"`.
3. Added Simplified Chinese (Pinyin) in Gboard settings. Typed `nihao`, the
   candidate strip showed `ni hao` / 你好, selecting 你好 produced exactly one
   `你好` in the editor. After Done the Rust payload was
   `code points: U+4F60 U+597D`. The egui label renders the two characters as
   `□□` because eframe's `default_fonts` has no CJK glyphs; the committed string
   itself is exact (verified by code points). A product build needs a CJK font.
4. `Clipboard set` → Chrome paste showed "TabTool clipboard" plus the system
   toast "Chrome pasted from your clipboard"; copying in Chrome and pressing
   `Clipboard get` returned `Some("TabTool clipboard")`. The forced
   `throwSecurityException` self-test still shows
   `SecurityException contained: true`, process alive.
5. `Open document` opened DocumentsUI filtered to `text/*`; picking
   `/sdcard/Download/sample.csv` yielded
   `content://com.android.providers.downloads.documents/document/msf%3A54`.
6. Keyboard: `keyboard` 0 ↔ 336 dp. Gesture navigation portrait:
   `top 24, bottom 24`; 3-button landscape: `top 24, right 48`; 3-button
   portrait: `top 24, bottom 48`. Rotation re-laid out the egui surface and the
   activity stayed alive.
7. With the editor open, the system back gesture closed it with
   `last outcome: cancelled` and the committed text unchanged. Rotating
   portrait↔landscape while the editor was open also cancelled the edit and
   kept the committed value; no crash. The manifest declares `navigation` in
   `configChanges` so switching navigation mode does not recreate the activity
   (without it the native surface came back black after the mode switch).

## Notes

- Last stable `androidx.core` (1.19.x) and AGP 9.x require compileSdk 37, so the
  Gradle 8.x / compileSdk 36 constraint pins AGP 8.13.2 and core-ktx 1.18.0.
- `games-activity` 4.4.2 extends `AppCompatActivity`, so the manifest uses
  `Theme.AppCompat.NoActionBar` and the project depends on appcompat.
- The AVD's Vulkan driver crashes inside wgpu adapter setup, so the app pins
  wgpu to its GL backend on Android and the emulator is started with
  `-gpu host`. Disabling guest Vulkan (`-feature -Vulkan`) instead made GL
  surface creation fail with `BadAlloc`.
- The overlay is positioned from the egui field rectangle in points multiplied
  by `pixels_per_point`. It sits in `android.R.id.content` above the
  `SurfaceView`, and its top-left matched the placeholder in every tested
  orientation.
- Because the editor commits on focus loss, switching apps or opening another
  Android activity commits the current text (expected for this spike).
- The spike app's `insets` and clipboard paths are unchanged from the first
  round; only the input path is new.

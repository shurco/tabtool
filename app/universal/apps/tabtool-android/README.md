# TabTool Android host

Real Android host for the Rust `tabtool-core` port: a Gradle project with a
`GameActivity` subclass, Kotlin JNI bridge, native `EditText` overlay, and a Rust
`cdylib` built with eframe 0.37.0 / egui 0.37.0.

The Rust side is the single-window mobile app: the documents library, one
editor, sheets, autosave, intents and process-death state. The JNI bridge lives
in `tabtool-platform::android`; `MainActivity` and `TabToolBridge` own the
Android lifecycle and the native text editor.

## Why text entry uses a native editor

winit 0.30.13's Android backend handles only `InputEvent::KeyEvent` and
`InputEvent::MotionEvent`; GameActivity's soft-keyboard commits arrive as
`android_activity::input::InputEvent::TextEvent` and are dropped before
`egui-winit`. A stock egui `TextEdit` therefore never receives IME text on this
stack (verified in the Plan 1 feasibility run).

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
| JDK | 17-21 (Gradle 8.x cannot run on the bundled JBR 25) |
| Emulator | `Medium_Phone_API_36.1` (arm64, Android 16), headless |

`androidx.core` 1.19.x requires compileSdk 37 and AGP 9.1, so 1.18.0 is the
newest version compatible with the mandated compileSdk 36 / Gradle 8.x.

## Build

Set the SDK, NDK and JDK first:

```sh
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export JAVA_HOME="/opt/homebrew/opt/openjdk@17"   # any JDK 17-21

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
by `tabtool-platform::android`. Results are `OK`, `OK:<value>`, `CANCELLED` or
`ERROR:<message>`; SAF calls return `null` while the picker or copy runs.

| Kotlin | Rust | Notes |
| --- | --- | --- |
| `clipboardGet(): String?` | `Clipboard::get` | `SecurityException` is caught in Kotlin and cleared in Rust |
| `clipboardSet(text: String)` | `Clipboard::set` | |
| `documentsDir(): String?` | `AndroidPlatform::documents_dir` | app-private `files/documents`, created on demand |
| `importDocument(): String?` | `AndroidPlatform::import_from_picker` | `ACTION_OPEN_DOCUMENT` (`text/*`); streams the URI into the documents dir under a collision-free name |
| `exportDocument(sourcePath, suggestedName): String?` | `AndroidPlatform::export_copy` | `ACTION_CREATE_DOCUMENT`; streams the source file to the chosen URI |
| `shareFile(path): String` | `AndroidPlatform::share` | `FileProvider` (`${applicationId}.fileprovider`) + `ACTION_SEND` chooser |
| `insets(): IntArray` | `AndroidPlatform::safe_area` / `keyboard_height` | `[left, top, right, bottom, keyboard]` in dp |
| `languages(): Array<String>` | `SystemInfo::languages` | system language tags, most preferred first |
| `openUrl(url): String` | `OpenUrl::open` | http, https and mailto only |
| `isImportPending()` / `isExportPending()` | `AndroidPlatform::import_pending` / `export_pending` | keeps the library and export sheet on their own polling paths |
| `takeBack(): Boolean` | `AndroidPlatform::take_back` | back is consumed and queued; the app exits at its root with `finishActivity` |
| `showTextEditor(left, top, width, height, text, multiline)` | `TextInput::open` | rectangle in points; Kotlin converts with the display density; returns false when it cannot open |
| `hideTextEditor()` | `TextInput::close` | removes the editor without committing or cancelling |
| `filesDir(): String?` | `AndroidPlatform::files_dir` | settings and `app-state.json` live here |
| `clock(): IntArray` | `AndroidPlatform::clock` | `[utc offset minutes, twelve-hour flag]` for library dates |
| `takeIntentImport(): String?` | `AndroidPlatform::take_intent_import` | `ACTION_VIEW`/`ACTION_SEND` copy staged under a hidden name |
| `takeBackgroundEvent(): Boolean` | `AndroidPlatform::take_background_event` | one transition to the background |
| `nativeWake()` | `tabtool-android` | wakes egui after a JNI commit, intent copy or SAF result |

`TabToolBridge.nativeTextCommitted(text)` / `nativeTextCancelled()` are the JNI
callbacks the editor uses. They store the outcome in the shared editor session,
which `TextInput::poll` drains. `nativeWake()` requests the next egui pass so a
commit lands before the app idles. A root view that disappears before the editor
opens reports a cancel, so `is_active` can never stick. Back cancels; the IME
Done action and focus loss commit; rotation cancels the edit without touching
the committed value. `nativeSaveOnBackground()` blocks the activity's pause path
for at most 400 ms while the app flushes the pending small-document save.

SAF, share and link failures are queued as `tabtool_i18n::LocalizedError
{ message_key, detail }` and drained with `AndroidPlatform::take_error`. The
app resolves
`message_key` through `tabtool-i18n`; `detail` stays in logcat.

## Platform services checklist (Plan 5 Task 1)

Verified on the `Medium_Phone_API_36.1` AVD (arm64, Android 16, headless,
`-gpu host`).

| # | Check | Result | Evidence |
| --- | --- | --- | --- |
| 1 | Import a CSV into the private directory | **PASS** | Picked `sample.csv` from Downloads; `run-as ... ls files/documents` showed `sample.csv`, 26 bytes; the UI showed the private path. |
| 2 | Export a byte-identical copy to Downloads | **PASS** | `ACTION_CREATE_DOCUMENT` opened with `TabTool export.csv`; `adb pull` + `cmp` matched the source; the working file stayed in the private directory. |
| 3 | Clipboard round-trip | **PASS** | `Clipboard set` then `Clipboard get` returned `Some("TabTool clipboard")`. |
| 4 | Share sheet opens with the document | **PASS** | The chooser (`com.android.intentresolver/.ChooserActivityLauncher`) showed "Sharing 1 file: sample.csv". |
| 5 | Insets report and react | **PASS** | Safe area `[0, 24, 0, 24]` dp in portrait; keyboard `0 ↔ 336` dp while editing; landscape re-laid out the surface. |
| 6 | Back closes the editor without committing | **PASS** | With the keyboard visible, the first back hides the IME; the next back produced `last outcome: cancelled` and kept the committed text. Later backs produced `status: back: taken` and the activity stayed alive. |
| 7 | Text input commit | **PASS** | `Platform` typed through Gboard committed once on IME Done. |
| 8 | System languages and links | **PASS** | `languages: ["en-US"]`; `Open URL` resumed Chrome. |
| 9 | JNI exception containment | **PASS** | Historical Plan 1 check; the current bridge still clears any pending JNI exception before logging. |
| 10 | Import cancel leaves no file and no stuck picker | **PASS** | Backing out of the picker left the documents dir unchanged and a second import opened normally. |

## Plan 1 feasibility checklist (historical)

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
   `Clipboard get` returned `Some("TabTool clipboard")`.
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
- The overlay is positioned from the egui field rectangle in points; Kotlin
  applies the display density and places it in `android.R.id.content` above the
  `SurfaceView`, and its top-left matched the placeholder in every tested
  orientation.
- Because the editor commits on focus loss, switching apps or opening another
  Android activity commits the current text.
- The platform services' insets and clipboard paths feed
  `tabtool-platform::android`; SAF import/export, share, languages, links, back,
  intents and the FileProvider are shared with the app.

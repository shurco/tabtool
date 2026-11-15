# Android emulator checklist

The acceptance pass for the Plan 5 Android app, run on the `Medium_Phone_API_36.1`
AVD (Android 16, arm64, 1080×2400, 420 dpi) on 2026-11-15. The build under test
was the debug APK from the final Task 8 tree; the library, document, theme,
rotation, process-death and back rows were re-checked after the last rebuild.

Every row records what was exercised and the evidence that was observed. A
`NOT VERIFIED` row says why it could not run here; it is not a pass.

| Check | Result | Evidence |
| --- | --- | --- |
| Create a document | PASS | `+` → New Document opened `Untitled.csv`; `files/documents/Untitled.csv` appeared. |
| Import through SAF | PASS | `+` → Import… → Downloads → `cjk-emoji.csv` opened; the private copy kept its name. |
| Import a name already taken | PASS | A second import of `cjk-emoji.csv` became `cjk-emoji 2.csv`; the first file was unchanged. |
| Cancel an import | PASS | Back in the system picker returned to the library; no new file appeared. |
| Edit with the native editor | PASS | Two taps on a cell opened the platform `EditText` over it; Gboard typing, IME Done and the editor accessory committed once. |
| Undo | PASS | The toolbar undo restored the edited cell and the file on disk. |
| Autosave | PASS | After a small edit the file contained `"Hello"` within the 0.8 s debounce, with no explicit save. |
| Save through Back | PASS | A dirty small document was written before the editor closed. |
| Save a large document | PASS | A 157.7 MB CSV was edited and saved from the Back prompt; the saved file contained the edit and all fields were re-quoted. |
| Cancel a large save | NOT VERIFIED | The status line showed `Saving… 53%` with Cancel on the 150 MiB file, but the save completed before the tap could cancel it (the control passed: Cancel was visible and enabled). Cancellation is covered by `tabtool-document` large-save tests. |
| Export through SAF | PASS | Export sheet → SAF create → `cjk-emoji 2-export.csv` in Downloads; content was `"名前","メモ"` with the CJK and emoji cells byte-for-byte. |
| Delete | PASS | Swipe left on a row, Delete, confirmation; the row and the private file were gone. |
| Dark appearance | PASS | `cmd uimode night yes`; the app and sheets switched to the dark token set and the status and navigation bar icons became light. |
| Light appearance | PASS | `cmd uimode night no`; the app switched back while running, and the status and navigation bar icons became dark. |
| 11 languages | PASS | Settings → Language was switched to English, Français, Español, Deutsch, Italiano, Polski, Беларуская, Українська, 简体中文, 日本語 and 한국어; every sheet label changed immediately with no restart hint. |
| CJK and emoji | PASS | Japanese, Simplified Chinese, Cyrillic, `😀` and `🎉` rendered from the system fonts in the grid, library and sheets. |
| Rotate | PASS | Landscape kept the document, row selection and the action bar; portrait restored the same selection. |
| Fold / hinge | NOT VERIFIED | No foldable AVD is installed (`emulator -list-avds` has `Medium_Phone_API_36.1` and `Crash_API_29`; the phone AVD reports `hw.sensor.hinge=no`). The folded layouts and hinge split are covered by `tests/mobile/visual.rs` and `tests/mobile/fold.rs`. |
| Background / foreground | PASS | With a dirty cell, Home saved through the background path; returning to the app showed the saved value. |
| Process death | PASS | `am force-stop` then relaunch restored `app-state.json`: the editor, the Settings sheet and its back order. Unsaved edits were not restored, as designed. |
| Share | PASS | Overflow → Share opened the system chooser for `cjk-emoji 2.csv` with Drive, Gmail and Quick Share targets. |
| Back order | PASS | Back closed Settings, then the editor, then exited to the launcher; a second Back at the library exits. |
| Large 1 GiB file | NOT VERIFIED | Not feasible on this machine: the 5.8 GB AVD data partition had about 1 GB free and the host volume about 2.7 GB, so a 1 GiB import plus its private copy and save temporary could not fit. The same code path was exercised with a 157.7 MB file (large mode, 4,600,000 rows, edit, save). |
| Icon | PASS | The launcher icon shows the white grid frame with the blue center cell inside the adaptive mask. |

## Notes from the run

- The emulator reports `No font has "こ"` once while the restored document is
  built, before `install_fonts` runs; the grid still renders CJK from system
  fonts. No user-visible fallback was observed.
- `am force-stop` followed immediately by a launch can leave the launcher
  splash visible for a few seconds while the activity is destroyed and
  recreated; waiting or using `am start` again recovers. No crash or ANR was
  recorded.
- The AVD data partition had to be freed (unused apps removed) before the
  second debug install; the app itself keeps no cache beyond its documents.

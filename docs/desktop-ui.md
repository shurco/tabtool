# TabTool desktop UI (Plan 4)

Plan 4 delivers the desktop editor for Windows and Linux, with a macOS
development build that follows the Apple layout and behavior. This document is
the manual parity checklist, the snapshot baseline inventory, the performance
evidence and the record of known deviations. It complements
`app/universal/docs/universal.md`.

The Apple app in `app/apple` remains the visual source of truth. The reference
screenshots are:

- `web/static/img/app-grid.webp` and `app-grid-1200.webp` — grid, toolbar,
  format bar, status line;
- `web/static/img/app-columns.webp` and `app-columns-1200.webp` — selected
  column and column menu;
- `web/static/img/app-large.webp` and `app-large-1200.webp` — large-file mode,
  indexing banner and limited commands.

The web images are 1200×750; the desktop baselines are 1200×800 and 640×400.
Compare layout, spacing, wording, control order and state, not the surrounding
OS window frame.

## Running the desktop app

From `app/universal`:

```sh
cargo run -p tabtool-desktop
cargo run -p tabtool-desktop -- path/to/file.csv
cargo run -p tabtool-desktop -- --smoke
cargo build -p tabtool-desktop --release
```

Windows has no extra setup. Linux needs X11 or Wayland, a Vulkan/GL driver and
the GUI development libraries installed for CI: `libxkbcommon-dev` and
`libwayland-dev`. File dialogs go through the XDG desktop portal over D-Bus
(`rfd`'s `xdg-portal` backend), so GTK development headers are not required.
`--smoke` opens no window and prints `tabtool 0.1.0`.

## Manual visual and behavior checklist

Run this pass on Windows 10/11 and on Linux under both X11 and Wayland before
a release. Record the OS, display scale and compositor next to each result.
The automated suites cover the same behavior headlessly; this pass is for the
native window, IME and compositor behavior they cannot reach.

**Verification status: NOT RUN on Windows 10/11; NOT RUN on Linux/X11; NOT RUN
on Linux/Wayland.** Every item below is unexecuted and explicitly marked
`NOT RUN`. The GitHub Actions workflow described here has not been run on
GitHub from this machine; its YAML is configuration, not execution evidence.
The macOS automated suites were run locally in this task.

### Launch and window lifecycle — NOT RUN

- [ ] **NOT RUN** — Launch with no arguments: one untitled document window opens with
      the full menu bar; after its last window closes, the root shows “No Documents,”
      the New Document button and the full menu bar.
- [ ] **NOT RUN** — Launch with a CSV path: one document window opens with the file name and
      the detected format; the root window stays hidden.
- [ ] **NOT RUN** — New, Open, Open Recent, Save, Save As, Revert and Export open
      the expected dialogs and update the title/dirty state.
- [ ] **NOT RUN** — Closing a dirty document asks Save / Don’t Save / Cancel; closing a clean
      document closes at once; quitting with several documents follows the same
      prompts.
- [ ] **NOT RUN** — Settings and About open as separate native windows, resize with their
      content and close without leaving the app in a busy state.
- [ ] **NOT RUN** — X11 and Wayland both create, move, focus and close the deferred document
      windows. If a compositor cannot create child viewports, confirm the
      embedded fallback still renders a usable document window and record it.

### Menus and shortcuts — NOT RUN

- [ ] **NOT RUN** — Menu bar order matches the Apple app: TabTool, File, Edit, Table, View,
      Help, with the documented items in each menu.
- [ ] **NOT RUN** — Unavailable commands are disabled and stay disabled after a state change
      (empty document, read error, large mode, filtered rows, running save).
- [ ] **NOT RUN** — Cmd on macOS and Ctrl on Windows/Linux activate New, Open, Save, Save As,
      Undo, Redo, Cut, Copy, Paste, Select All, Find/Filter, Export and Quit.
- [ ] **NOT RUN** — Menu-initiated Paste still asks the platform for the clipboard text and
      applies it when the paste event arrives.
- [ ] **NOT RUN** — Large mode disables Select All, structural rows/columns, Sort, Filter,
      decimal-mark changes and Export.

### IME and text entry — NOT RUN

- [ ] **NOT RUN** — Type Latin, Cyrillic and CJK text into a cell, a column header rename and
      the filter value field.
- [ ] **NOT RUN** — The IME candidate window follows the caret; composition text is visible
      and committing with Enter does not also move to the next cell.
- [ ] **NOT RUN** — Escape cancels the cell edit; clicking another cell commits exactly once;
      Tab/Shift+Tab commit and move.
- [ ] **NOT RUN** — Paste multi-cell TSV from a spreadsheet; undo restores the previous cells.
- [ ] **NOT RUN** — Filter values, settings fields and the About window accept the same input
      methods.

### Grid, chrome and panels — NOT RUN

- [ ] **NOT RUN** — Row and column selection, range selection, the 2 pt focus ring and the
      row-number gutter match `app-grid` and `app-columns`.
- [ ] **NOT RUN** — Header sort chevrons, alternating rows, row height and hidden row numbers
      follow the Grid settings.
- [ ] **NOT RUN** — Column resize, row/column drag inside the grid, and drag cancellation
      outside the grid behave as documented.
- [ ] **NOT RUN** — The filter bar shows one row per criterion, Add/Remove keep at least one
      row, Clear resets, Apply/Apply All change rows, Hide keeps the draft.
- [ ] **NOT RUN** — The format bar order is Encoding, Separator, Quote, Decimal, Header and
      the controls disable while busy or in large mode.
- [ ] **NOT RUN** — Needs-attention, save-error, format-alert, reparse confirmation, notice
      and read-error overlay match their committed baselines.
- [ ] **NOT RUN** — Settings shows all four tabs with the documented defaults and applies a
      change immediately. About shows the version, license, website and Rate
      button. Export shows the format options and the header note.

### Files, drag and drop — NOT RUN

- [ ] **NOT RUN** — Drop one CSV on the root window and on a document window: each opens a
      document.
- [ ] **NOT RUN** — Drop several files: each opens in its own window.
- [ ] **NOT RUN** — Drop an unreadable table file: the typed error is shown without
      losing the current document. A dropped directory is ignored because only
      table-file extensions open.
- [ ] **NOT RUN** — Drag a row or column inside the grid: the drop indicator appears and the
      move lands in the expected slot.
- [ ] **NOT RUN** — Drag out of the window: known deviation, only internal drags are
      accepted.
- [ ] **NOT RUN** — Save As to an existing file asks before replacing it; Export writes the
      converted copy and reports the write error on failure.

### Large files (1 GiB) — NOT RUN

- [ ] **NOT RUN** — Open a 1 GiB CSV: the window appears before indexing completes, shows
      “Indexing…”, the known row count and the large-mode banner, then settles
      at 100%.
- [ ] **NOT RUN** — Scroll and jump to the end: no blocking, no rows beyond the indexed
      region, and memory stays well below the file size.
- [ ] **NOT RUN** — Edit cells, save, and confirm the progress bar and Cancel; cancel leaves
      the original file untouched.
- [ ] **NOT RUN** — Append a row outside the app: the stale banner appears without a user
      action; Reload shows the new row.
- [ ] **NOT RUN** — Confirm the unavailable commands are disabled in large mode.

### Themes and languages — NOT RUN

- [ ] **NOT RUN** — Light and dark appearance match the committed baselines on every
      surface: grid, banners, format bar, filter bar, Settings, About, Export.
- [ ] **NOT RUN** — Switching appearance while a document is open repaints it immediately.
- [ ] **NOT RUN** — Every one of the 11 languages (`en`, `fr`, `es`, `de`, `it`, `pl`,
      `be`, `uk`, `zh-Hans`, `ja`, `ko`) renders the menu bar, toolbar,
      banners, dialogs, Settings, About and Export with no clipped label and no
      leftover English except product names, URLs and the untitled name.
- [ ] **NOT RUN** — System language follows the OS locale; an explicit choice survives a
      restart.

### Accessibility basics — NOT RUN

- [ ] **NOT RUN** — Tab reaches every control in document order and the focus ring is
      visible; Shift+Tab reverses; Escape leaves menus and modals.
- [ ] **NOT RUN** — A screen reader announces each button, toggle, picker and text field
      with its visible or accessible name; disabled controls report disabled.
- [ ] **NOT RUN** — Grid headers, cells, the row-number gutter, the filter bar and the
      Settings tabs expose row/column and role information.
- [ ] **NOT RUN** — High-contrast/system accessibility settings keep text and focus visible.

## Fonts

TabTool bundles no fonts and ships no font bytes in the desktop executable. At
startup `tabtool-ui` installs the operating system's generic families in a
fixed order:

- proportional: system sans-serif → system serif → system monospace;
- monospace: system monospace → system sans-serif → system serif;
- the named `UI Heading` family used by titles: the proportional order.

`egui` 0.37's `egui_system_font_provider` then discovers a platform font for
each character the installed faces lack. That provider covers Chinese,
Japanese, Korean, Cyrillic, Greek and emoji from the OS (for example Apple
Color Emoji on macOS, Segoe UI Emoji on Windows, Noto Color Emoji on Linux when
installed). Font files are memory-mapped, not embedded, and no monochrome emoji
font is bundled: platforms without a color emoji font fall back to their
available glyphs or tofu.

On the first frame the desktop app checks representative text for Chinese `中`,
Japanese `あ`, Korean `한`, Cyrillic `Привет`, Greek `Ελλάδα` and emoji `😀`.
For every missing script it prints one warning on stderr that names the script
and the affected UI language (Belarusian and Ukrainian for Cyrillic; Greek and
emoji only appear in CSV content). The check never stops startup and never
blocks rendering: whatever the OS provides is what the app shows.

## Snapshot policy

Image comparison is opt-in and local. The operating system's font files and the
GPU rasterizer differ between machines, so a PNG snapshot is only valid on the
machine that rendered it. No image baseline is committed, and images are not a
CI gate on macOS, Linux or Windows.

The consolidated Plan 4 test in
`app/universal/apps/tabtool-desktop/tests/visual.rs` still renders 22 states at
1200×800 and 640×400 in light and dark, and writes
`<state>-<size>-<theme>.png` only when enabled. The table below is the scenario
inventory, not a set of committed images.

| State | What the image pins |
| --- | --- |
| `welcome` | Root empty state and menu bar |
| `new-document` | Untitled 1×3 document, toolbar, format bar, status |
| `csv-grid-header` | Header row, data, sort chevrons, status count |
| `selected-row` | Row selection highlight |
| `selected-column` | Column selection highlight |
| `editing-cell` | Inline cell editor and caret |
| `format-bar` | Semicolon separator and decimal-comma controls, distinct from the default header grid |
| `filter-bar` | Criterion row, hints, Clear/Apply All |
| `needs-attention` | Write-gate banner with Undo/Format Controls |
| `read-error` | Full-area overlay and Detect Automatically |
| `save-error` | Failed-save banner above the format bar |
| `format-alert` | One-shot format failure banner |
| `reparse-confirm` | Discard confirmation before a re-read |
| `large-indexing` | Banner, progress bar, status and disabled commands |
| `saving` | Centered save progress, disabled format bar, Cancel |
| `stale` | Appended-file banner and Reload |
| `settings-general`, `settings-new-document`, `settings-grid`, `settings-large-files` | The four tabs and their defaults |
| `about` | Identity, links, MIT license |
| `export` | Export options modal |

Settings, About and Export are component-window scenarios: the content is
rendered in a 1200×800 or 640×400 harness so both documented sizes can be
checked; the OS-drawn window frame is not part of the image. Selecting a
Settings tab requests a content size, so the harness restores the requested
size before the snapshot; the 640×400 and 1200×800 files are not
interchangeable.

Commands, from `app/universal`:

```sh
# Skip image tests; everything else still runs on every platform.
cargo test --workspace

# Generate local baselines for this machine, then compare.
TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 \
    cargo test -p tabtool-desktop --test visual
TABTOOL_RENDER_SNAPSHOTS=1 cargo test -p tabtool-desktop --test visual

# The same policy for the granular widget, grid and chrome snapshots.
TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 cargo test -p tabtool-ui
TABTOOL_RENDER_SNAPSHOTS=1 cargo test -p tabtool-ui
```

Without `TABTOOL_RENDER_SNAPSHOTS=1`, image tests print one skip notice and
return. Review every changed image before keeping it: `UPDATE_SNAPSHOTS=1`
rewrites files, and a layout regression would otherwise be accepted silently.
The local comparison budget is `threshold = 2.0`, `max_failed_pixels = 4` in
`apps/tabtool-desktop/kittest.toml` and `crates/tabtool-ui/kittest.toml`; do
not raise it to hide a layout change.

Baselines are per machine and per OS version. Regenerate them after changing
the OS, display scale, GPU driver, font set or theme. Do not add them to the
repository and do not make them a CI gate; scenario, interaction and
accessibility tests are the cross-platform gates. Linux CI runs the scenario
suite under `xvfb-run`; Windows and macOS run it directly.

## Performance evidence

The grid virtualization tests instrument `RowSource` with a call counter and
record the inclusive bounds read. They are correctness assertions, not
wall-clock gates: CI does not fail on a slow machine.

| Case | Size | Cells read | Debug test build | Release test build |
| --- | --- | --- | --- | --- |
| Scroll to row 99,000, 100k rows × 12 columns | 640×400 | 119 (17 rows × 7 columns) | 1.36 ms | 78 µs |
| Large document, 5,000 indexed rows × 3 columns | 640×400 | 48 (16 rows × 3 columns) | 741 µs | 59 µs |
| Partial publication, 120 of 5,000 rows | 640×400 | 48 (16 rows × 3 columns) | 882 µs | 48 µs |

The first-frame 1M-row × 20-column case reads exactly 112 cells (16×7) in the
same deterministic test; it asserts the bounds rather than a duration.

Measured on an Apple M4 Pro (24 GB) running macOS 26.6.2, Rust 1.99.1, with the
test harness's wgpu renderer. The 100k case has a 17-row bounds because the
grid keeps one row of overscan above the viewport; the viewport itself is
624×384 at scroll offset `(0, 99,000 × 24)`. The call budget is therefore
17 rows × 7 columns = 119 cells, and the test fails if the count or the bounds
change. No wall-clock assertion is made.

The large-document test asserts that every requested row is inside the
published index (`max_row < row_count`) and every requested column is inside
the widest scanned row (`max_column < max_field_count`). A second test models a
partial publication by wrapping the same real mapped source and reporting only
120 of its 5,000 rows as published; it asserts that the grid reads only rows
`0..=15` and never crosses the 120-row boundary. This is a fake partial index
(the test wrapper publishes the shorter row count), not a paused real indexer.
The source-level rule — the row still being scanned stays hidden — is pinned
separately by `tabtool-large`'s
`partial_index_hides_the_row_still_being_scanned`. The grid only asks for
indexes below the `row_count` it is given, and the large source derives that
count from `scanned_bytes`/`row_count`.

Run the evidence:

```sh
cargo test -p tabtool-ui --test grid render:: -- --nocapture
cargo test --release -p tabtool-ui --test grid render::a_hundred -- --nocapture
cargo test --release -p tabtool-ui --test grid render::a_partially -- --nocapture
```

The benchmark for real files (`cargo run -p xtask -- bench-large 1`) remains
the place for file-scoped throughput and memory numbers; see
`app/universal/docs/universal.md`.

## Design deviations (design §14)

These are the deliberate differences in design §14. A new deliberate difference
requires a design note and approval before implementation. The only item here
without completed approval is §14.18, which is implemented but pending user
approval.

| §14 | Deliberate difference | Status |
| --- | --- | --- |
| 14.1 | In-window menu bar on Windows/Linux instead of a native OS menu bar; shortcuts use Ctrl instead of Cmd. | Approved |
| 14.2 | No drag-out of rows to other applications; internal drags carry a TSV payload but only internal drops are accepted. | Approved |
| 14.3 | No StoreKit review prompt; About keeps a Rate link to the App Store listing. | Approved |
| 14.4 | Desktop autosave uses the iOS 0.8 s debounce instead of system-driven macOS autosave; save gates, banners and dirty semantics are identical. | Approved |
| 14.5 | Document type names, `Untitled` and `-export` stay English. | Approved |
| 14.6 | Emoji may render monochrome on Android when no system color rasterizer is available; glyphs and layout are unchanged. | Approved (Android) |
| 14.7 | Accessibility exposes roles, names and keyboard navigation, but does not claim VoiceOver-grade table semantics on desktop/Android in v1. | Approved |
| 14.8 | Multi-window uses eframe viewports; the fallback is one window with document tabs if a Linux compositor makes them unreliable. The fallback requires re-approval before implementation. | Approved |
| 14.9 | No native menu role integration, system Services or Finder/Dock behavior. | Approved (Apple-only) |
| 14.10 | Alternating row colors default is off, following the code and tests rather than the README table. | Approved |
| 14.11 | Android Save As writes a copy through SAF and keeps the editor on its private working file; iOS rebinds the document. | Approved (Android) |
| 14.12 | No bidi reordering or complex-script shaping (Arabic, Hebrew, Indic, Thai) in v1; text is stored and saved unchanged. | Approved |
| 14.13 | Fixed visual baseline is the compact macOS look from `web/static/img/app-*.webp`, not a system-version-dependent style. | Approved |
| 14.14 | Android text entry uses a native `EditText` overlay; desktop uses egui's `TextEdit`. | Approved 2026-11-12 |
| 14.15 | Sorting uses ICU4X collation; small locale-specific order differences from Foundation are pinned as known divergences. | Approved 2026-11-12 |
| 14.16 | A no-quote save whose first field begins with U+FEFF is blocked as ambiguous output instead of allowing silent data loss. | Approved 2026-11-12 |
| 14.17 | ISO-2022-JP decoding is stricter than Foundation for malformed input; the exact byte set is pinned by a golden test. | Approved 2026-11-12 |
| 14.18 | Select All is disabled in large-file mode because a `BTreeSet` of tens of millions of rows would freeze or exhaust memory; in-memory documents keep Select All. | **Pending user approval** |

Large mode also keeps the Apple capability restrictions: no structural edits,
sorting, filters, Export, decimal-mark changes, or Cut/Paste/Delete; Copy is
limited to 1,000,000 cells.

## Known gaps and observations (not §14)

These are not approved design differences. They are implementation gaps or
notes for the manual pass and must be fixed or promoted to §14 before release.

| Area | Gap or observation |
| --- | --- |
| Drop target | No blue drop-target highlight while files are dragged over a window. This is a visual-parity gap, not a design difference. |
| Header rename | Double-clicking a header does not start renaming; only the context-menu and Table-menu entries do. The brief specifies the menus, but this is still a macOS parity gap. |
| Duplicate | The Apple File menu's Duplicate command is not implemented in the desktop port and is not offered in the menu. |
| Dropped folders | A dropped directory is ignored because opening is gated on table-file extensions; the Apple app also only opens files. |
| Settings timing | Settings apply immediately; the Apple README mentions a restart for some preferences. This behavior is not in §14. |
| Native dialogs | Native file dialogs use the platform locale; the app language applies to in-app chrome created after a language switch. |
| Status line | A grid mutation reaches the status line on the next frame because the bottom panel is laid out before the central grid; it settles immediately and is covered by tests. |
| Linux viewports | Deferred viewport behavior on X11/Wayland is unverified manually. The embedded fallback keeps a document usable if child viewports are unavailable; the manual pass records which path was used. |
| Baselines | The 88 consolidated images are macOS/Metal only (about 3.9 MB). Linux/Windows must not compare them until per-OS baselines exist. |

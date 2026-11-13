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
the GUI development libraries installed for CI: `libxkbcommon-dev`,
`libwayland-dev` and `libgtk-3-dev`. `--smoke` opens no window and prints
`tabtool 0.1.0`.

## Manual visual and behavior checklist

Run this pass on Windows 10/11 and on Linux under both X11 and Wayland before
a release. Record the OS, display scale and compositor next to each result.
The automated suites cover the same behavior headlessly; this pass is for the
native window, IME and compositor behavior they cannot reach.

### Launch and window lifecycle

- [ ] Launch with no arguments: the root window shows “No Documents”, the
      New Document button, and the full menu bar.
- [ ] Launch with a CSV path: one document window opens with the file name and
      the detected format; the root window stays hidden.
- [ ] New, Open, Open Recent, Save, Save As, Revert, Duplicate and Export open
      the expected dialogs and update the title/dirty state.
- [ ] Closing a dirty document asks Save / Don’t Save / Cancel; closing a clean
      document closes at once; quitting with several documents follows the same
      prompts.
- [ ] Settings and About open as separate native windows, resize with their
      content and close without leaving the app in a busy state.
- [ ] X11 and Wayland both create, move, focus and close the deferred document
      windows. If a compositor cannot create child viewports, confirm the
      embedded fallback still renders a usable document window and record it.

### Menus and shortcuts

- [ ] Menu bar order matches the Apple app: TabTool, File, Edit, Table, View,
      Help, with the documented items in each menu.
- [ ] Unavailable commands are disabled and stay disabled after a state change
      (empty document, read error, large mode, filtered rows, running save).
- [ ] Cmd on macOS and Ctrl on Windows/Linux activate New, Open, Save, Save As,
      Undo, Redo, Cut, Copy, Paste, Select All, Find/Filter, Export and Quit.
- [ ] Menu-initiated Paste still asks the platform for the clipboard text and
      applies it when the paste event arrives.
- [ ] Large mode disables Select All, structural rows/columns, Sort, Filter,
      decimal-mark changes, Export and Duplicate.

### IME and text entry

- [ ] Type Latin, Cyrillic and CJK text into a cell, a column header rename and
      the filter value field.
- [ ] The IME candidate window follows the caret; composition text is visible
      and committing with Enter does not also move to the next cell.
- [ ] Escape cancels the cell edit; clicking another cell commits exactly once;
      Tab/Shift+Tab commit and move.
- [ ] Paste multi-cell TSV from a spreadsheet; undo restores the previous cells.
- [ ] Filter values, settings fields and the About window accept the same input
      methods.

### Grid, chrome and panels

- [ ] Row and column selection, range selection, the 2 pt focus ring and the
      row-number gutter match `app-grid` and `app-columns`.
- [ ] Header sort chevrons, alternating rows, row height and hidden row numbers
      follow the Grid settings.
- [ ] Column resize, row/column drag inside the grid, and drag cancellation
      outside the grid behave as documented.
- [ ] The filter bar shows one row per criterion, Add/Remove keep at least one
      row, Clear resets, Apply/Apply All change rows, Hide keeps the draft.
- [ ] The format bar order is Encoding, Separator, Quote, Decimal, Header and
      the controls disable while busy or in large mode.
- [ ] Needs-attention, save-error, format-alert, reparse confirmation, notice
      and read-error overlay match their committed baselines.
- [ ] Settings shows all four tabs with the documented defaults and applies a
      change immediately. About shows the version, license, website and Rate
      button. Export shows the format options and the header note.

### Files, drag and drop

- [ ] Drop one CSV on the root window and on a document window: each opens a
      document.
- [ ] Drop several files: each opens in its own window.
- [ ] Drop a directory or an unreadable file: the typed error is shown without
      losing the current document.
- [ ] Drag a row or column inside the grid: the drop indicator appears and the
      move lands in the expected slot.
- [ ] Drag out of the window: known deviation, only internal drags are
      accepted.
- [ ] Save As to an existing file asks before replacing it; Export writes the
      converted copy and reports the write error on failure.

### Large files (1 GiB)

- [ ] Open a 1 GiB CSV: the window appears before indexing completes, shows
      “Indexing…”, the known row count and the large-mode banner, then settles
      at 100%.
- [ ] Scroll and jump to the end: no blocking, no rows beyond the indexed
      region, and memory stays well below the file size.
- [ ] Edit cells, save, and confirm the progress bar and Cancel; cancel leaves
      the original file untouched.
- [ ] Append a row outside the app: the stale banner appears without a user
      action; Reload shows the new row.
- [ ] Confirm the unavailable commands are disabled in large mode.

### Themes and languages

- [ ] Light and dark appearance match the committed baselines on every
      surface: grid, banners, format bar, filter bar, Settings, About, Export.
- [ ] Switching appearance while a document is open repaints it immediately.
- [ ] Every one of the 11 languages (`en`, `fr`, `es`, `de`, `it`, `pl`,
      `be`, `uk`, `zh-Hans`, `ja`, `ko`) renders the menu bar, toolbar,
      banners, dialogs, Settings, About and Export with no clipped label and no
      leftover English except product names, URLs and the untitled name.
- [ ] System language follows the OS locale; an explicit choice survives a
      restart.

### Accessibility basics

- [ ] Tab reaches every control in document order and the focus ring is
      visible; Shift+Tab reverses; Escape leaves menus and modals.
- [ ] A screen reader announces each button, toggle, picker and text field
      with its visible or accessible name; disabled controls report disabled.
- [ ] Grid headers, cells, the row-number gutter, the filter bar and the
      Settings tabs expose row/column and role information.
- [ ] High-contrast/system accessibility settings keep text and focus visible.

## Snapshot baselines

The consolidated Plan 4 images live in
`app/universal/apps/tabtool-desktop/tests/snapshots/` and are named
`<state>-<size>-<theme>.png`. There are 88 images: 22 states × 2 sizes
(1200×800, 640×400) × 2 appearances (light, dark).

| State | What the image pins |
| --- | --- |
| `welcome` | Root empty state and menu bar |
| `new-document` | Untitled 1×3 document, toolbar, format bar, status |
| `csv-grid-header` | Header row, data, sort chevrons, status count |
| `selected-row` | Row selection highlight |
| `selected-column` | Column selection highlight |
| `editing-cell` | Inline cell editor and caret |
| `format-bar` | Encoding/separator/quote/decimal/header controls |
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

Settings, About and Export are component-window baselines: the content is
rendered in a 1200×800 or 640×400 harness so both documented sizes are pinned;
the OS-drawn window frame is not part of the image.

Commands, from `app/universal`:

```sh
cargo test -p tabtool-desktop --test visual       # compare on macOS
UPDATE_SNAPSHOTS=1 cargo test -p tabtool-desktop --test visual
TABTOOL_SKIP_SNAPSHOTS=1 cargo test -p tabtool-desktop --test visual
cargo test -p tabtool-ui                          # granular UI baselines
```

Review every changed image before committing it: `UPDATE_SNAPSHOTS=1` rewrites
files, and a layout regression would otherwise be accepted silently. The
comparison budget is `threshold = 2.0`, `max_failed_pixels = 4` in
`apps/tabtool-desktop/kittest.toml`; do not raise it to hide a layout change.

The older granular baselines under `crates/tabtool-ui/tests/snapshots/` remain
in place for widget, grid and document-chrome regressions. Both sets are
macOS-only.

### Per-OS baselines

The committed images were rendered on macOS 26.6.2 with Metal (Apple M4 Pro),
`pixels_per_point = 1.0`, and the bundled Inter, Noto Sans SC, Noto Sans KR and
Noto Emoji fonts. Linux and Windows use different GPU backends and font
rasterizers, so they must not compare these images until they have their own
baselines.

To enable per-OS comparison later:

1. render and review the set on the target OS with `UPDATE_SNAPSHOTS=1`;
2. store it under a per-OS path (for example
   `tests/snapshots/<os>/<state>-<size>-<theme>.png`) or add `[windows]` /
   `[linux]` entries to `kittest.toml`;
3. remove the macOS-only `cfg` from the image test and point it at the per-OS
   path;
4. keep scenario, interaction and accessibility tests on all three jobs.

Until then the CI split is: macOS compares images; Linux runs the scenario
suite under `xvfb-run`; Windows runs it directly. Image comparison stays off
on runners without a usable GPU.

## Performance evidence

The grid virtualization tests instrument `RowSource` with a call counter and
record the inclusive bounds read. They are correctness assertions, not
wall-clock gates: CI does not fail on a slow machine.

| Case | Size | Cells read | Debug test build | Release test build |
| --- | --- | --- | --- | --- |
| Scroll to row 99,000, 100k rows × 12 columns | 640×400 | 119 (17 rows × 7 columns) | 1.36 ms | 78 µs |
| Large document, 5,000 indexed rows × 3 columns | 640×400 | 48 (16 rows × 3 columns) | 741 µs | 59 µs |

The first-frame 1M-row × 20-column case reads exactly 112 cells (16×7) in the
same deterministic test; it asserts the bounds rather than a duration.

Measured on an Apple M4 Pro (24 GB) running macOS 26.6.2, Rust 1.99.1, with the
test harness's wgpu renderer. The 100k case has a 17-row bounds because the
grid keeps one row of overscan above the viewport; the viewport itself is
624×384 at scroll offset `(0, 99,000 × 24)`. The call budget is therefore
17 rows × 7 columns = 119 cells, and the test fails if the count or the bounds
change. No wall-clock assertion is made.

The large-document test also asserts that every requested row is inside the
published index (`max_row < row_count`) and every requested column is inside
the widest scanned row (`max_column < max_field_count`). The large source
reports only rows whose end is known: while indexing, the last row still being
scanned is hidden, and `row_count` is derived from the indexer's
`scanned_bytes`/`row_count` counters. The UI therefore cannot request a row
that the scan has not published.

Run the evidence:

```sh
cargo test -p tabtool-ui --test grid render:: -- --nocapture
cargo test --release -p tabtool-ui --test grid render::a_hundred -- --nocapture
```

The benchmark for real files (`cargo run -p xtask -- bench-large 1`) remains
the place for file-scoped throughput and memory numbers; see
`app/universal/docs/universal.md`.

## Known deviations

These are deliberate and reviewed. A new deviation needs a design note and a
test or a manual checklist item.

| Area | Deviation |
| --- | --- |
| Menu bar | The menu is drawn inside each viewport instead of using the macOS system menu bar. Windows/Linux need an in-window menu, and one implementation keeps the platforms consistent. |
| Drag out | Dragging rows out of the window is not implemented; internal row/column drags carry TSV but only internal drops are accepted. |
| Drop target | There is no blue drop-target highlight while files are dragged over a window. |
| Header rename | Double-clicking a header does not start renaming; the context menu entry is the only path (the brief specifies the menu). |
| Select All | Select All is disabled in large mode. The Apple app can materialize an enormous selection; the port refuses it (design §14.18). |
| Filter hints | The filter-bar shortcut hints show ⌘ glyphs on every platform even where the bound modifier is Ctrl. The shortcuts themselves follow the platform modifier. |
| Collation | Sorting uses ICU4X, so a small number of strings order differently from Apple's Foundation. The cases are pinned in `testdata/golden/known_divergences.json`. |
| Settings timing | Settings apply immediately; the Apple README mentions a restart for some preferences. |
| Native viewports | Deferred viewports depend on the Linux compositor. The embedded fallback keeps a document usable if child viewports are unavailable; the manual pass records which path was used. |
| Rate link | About opens the App Store review URL on every platform; the desktop release has no store listing yet. |
| Dialogs | Native file dialogs use the platform locale; the app language applies to the in-app chrome created after a language switch. |
| Status line | A grid mutation is reflected in the status line on the next frame because the bottom panel is laid out before the central grid. It settles immediately and is covered by tests. |

Large mode also keeps the Apple capability restrictions: no structural edits,
sorting, filters, Export, decimal-mark changes, or Cut/Paste/Delete; Copy is
limited to 1,000,000 cells.

# Desktop visual parity baselines

These are the consolidated Plan 4 baselines rendered by
`apps/tabtool-desktop/tests/visual.rs` with egui_kittest's wgpu renderer on
macOS/Metal at `pixels_per_point = 1.0`, using the bundled Inter, Noto Sans SC,
Noto Sans KR and Noto Emoji fonts.

Every state is stored as `<state>-<size>-<theme>.png`:

- sizes: `1200x800` (default window) and `640x400` (minimum window);
- themes: `light` and `dark`;
- states: `welcome`, `new-document`, `csv-grid-header`, `selected-row`,
  `selected-column`, `editing-cell`, `format-bar`, `filter-bar`,
  `needs-attention`, `read-error`, `save-error`, `format-alert`,
  `reparse-confirm`, `large-indexing`, `saving`, `stale`, the four
  `settings-*` tabs, `about` and `export`.

The Settings, About and Export images are component-window baselines: the
content is rendered inside a `1200×800` or `640×400` harness so the layout can
be compared at both documented sizes, not inside an OS-drawn window frame.

`../../kittest.toml` sets the comparison budget (threshold 2.0, at most 4
different pixels). Do not raise it to hide a layout change.

Set `UPDATE_SNAPSHOTS=1` to rewrite the images after reviewing a deliberate
visual change. Set `TABTOOL_SKIP_SNAPSHOTS=1` to skip image comparison on a
macOS machine without a usable Metal device.

The image comparison is macOS-only. Windows and Linux CI run the scenario,
interaction and accessibility suites; the desktop baselines must be regenerated
and reviewed on each OS before those jobs compare them. Until per-OS baselines
exist, do not run this test on Windows/Linux, where font rasterization and the
GPU backend differ.

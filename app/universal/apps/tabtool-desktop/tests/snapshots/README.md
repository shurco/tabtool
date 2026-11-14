# Desktop visual snapshots

No image baseline is committed. TabTool uses the operating system's fonts, and
the rasterizer differs between OS versions, GPU drivers and font versions, so a
committed PNG would only be valid on the machine that rendered it. Image
comparison is therefore opt-in and local, never a CI gate.

`apps/tabtool-desktop/tests/visual.rs` still renders the consolidated Plan 4
inventory:

- sizes: `1200x800` (default window) and `640x400` (minimum window);
- themes: `light` and `dark`;
- states: `welcome`, `new-document`, `csv-grid-header`, `selected-row`,
  `selected-column`, `editing-cell`, `format-bar`, `filter-bar`,
  `needs-attention`, `read-error`, `save-error`, `format-alert`,
  `reparse-confirm`, `large-indexing`, `saving`, `stale`, the four
  `settings-*` tabs, `about` and `export`.

The Settings, About and Export images are component-window renderings: the
content is rendered inside a `1200×800` or `640×400` harness so the layout can
be compared at both documented sizes, not inside an OS-drawn window frame. A
Settings tab click requests a content resize, so the harness restores the
requested baseline size before the snapshot; the 640×400 and 1200×800 Settings
files are not interchangeable.

From `app/universal`, generate baselines for the current machine:

```sh
TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 \
    cargo test -p tabtool-desktop --test visual
```

Then compare after a reviewed change:

```sh
TABTOOL_RENDER_SNAPSHOTS=1 cargo test -p tabtool-desktop --test visual
```

Without `TABTOOL_RENDER_SNAPSHOTS=1` the image tests print a skip notice and
return; the scenario, interaction and accessibility suites always run on every
platform. Review every changed image before keeping it, and regenerate after a
font, theme, scale or OS change. `../../kittest.toml` sets the local comparison
budget (threshold 2.0, at most 4 different pixels); do not raise it to hide a
layout change.

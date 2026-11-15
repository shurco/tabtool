# Granular UI snapshots

No image baseline is committed. TabTool uses the operating system's fonts, and
the glyph rasterizer depends on the OS, GPU and installed font versions, so an
image is only meaningful on the machine that rendered it. Image comparison is
opt-in and local, never a CI gate.

These are the granular widget, grid and document-chrome snapshots. The
consolidated Plan 4 state inventory lives in
`apps/tabtool-desktop/tests/snapshots/` and is rendered by
`apps/tabtool-desktop/tests/visual.rs`. The consolidated Plan 5 mobile
inventory is rendered by `tests/mobile/visual.rs` and shares this directory:
the library, editor, action bars, sheets and large-file states on the phone,
tablet and foldable sizes.

Generate baselines for the current machine:

```sh
TABTOOL_RENDER_SNAPSHOTS=1 UPDATE_SNAPSHOTS=1 cargo test -p tabtool-ui
```

Then compare:

```sh
TABTOOL_RENDER_SNAPSHOTS=1 cargo test -p tabtool-ui
```

Without `TABTOOL_RENDER_SNAPSHOTS=1` the image tests print a skip notice and
return. Scenario, interaction and accessibility tests run on every platform
without a flag. Generated PNGs are ignored by `.gitignore`
(`app/universal/**/tests/snapshots/*.png`) and CI has no visual gate, so review
them locally before relying on a change. `kittest.toml` allows a small local
difference (threshold 2.0, budget 4 pixels); do not raise it to hide a layout
change.

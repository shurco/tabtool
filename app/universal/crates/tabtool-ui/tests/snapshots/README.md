# Snapshot baselines

These images were rendered with egui_kittest's wgpu renderer on macOS/Metal at
`pixels_per_point = 1.0`, using the bundled Inter, Noto Sans SC and Noto Emoji
fonts.

The image tests run only on macOS. Set `TABTOOL_SKIP_SNAPSHOTS=1` to skip them
on a machine without a suitable GPU. Scenario, interaction and accessibility
tests run on every platform.

`kittest.toml` allows a small per-pixel difference (threshold 2.0, budget 4
pixels) because GPU backends and font rasterizers may antialias slightly
differently. Do not raise it to hide a layout change.

These are the granular UI baselines. The consolidated Plan 4 state inventory
lives in `apps/tabtool-desktop/tests/snapshots/` and is rendered by
`apps/tabtool-desktop/tests/visual.rs`. Both sets remain macOS/Metal baselines;
per-OS baselines (or a per-OS output path) are required before image tests run
on Windows/Linux CI. See `docs/desktop-ui.md`. Do not delete these macOS
baselines.

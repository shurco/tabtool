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

Task 9 must produce per-OS baselines (or a per-OS output path) before the image
tests run on Windows/Linux CI. Do not delete these macOS baselines.

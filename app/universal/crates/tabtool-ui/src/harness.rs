//! The deterministic `egui_kittest` harness used by every UI test.
//!
//! A harness gets the bundled fonts, a fixed 1200×800 (or 640×400) viewport,
//! `pixels_per_point = 1.0`, the chosen desktop theme and wgpu rendering, so
//! snapshots do not depend on the machine's fonts or display scale.

use core::ops::{Deref, DerefMut};

use egui::{Context, Ui, Vec2};
use egui_kittest::Harness;

pub use egui_kittest::kittest;

use crate::{Theme, apply_theme, install_fonts};

/// A kittest harness with TabTool fonts and theme installed.
pub struct UiHarness<'a> {
    harness: Harness<'a, ()>,
    theme: Theme,
}

impl<'a> UiHarness<'a> {
    /// The default window size.
    pub const WIDE: [f32; 2] = [1200.0, 800.0];
    /// The minimum desktop window size.
    pub const COMPACT: [f32; 2] = [640.0, 400.0];

    /// Build a harness of `size` logical points with `app` as the root UI.
    pub fn new(theme: Theme, size: [f32; 2], app: impl FnMut(&mut Ui) + 'a) -> Self {
        let harness = Harness::builder()
            .with_size(Vec2::new(size[0], size[1]))
            .with_pixels_per_point(1.0)
            .with_theme(theme.mode.egui())
            .with_context_setup(move |ctx: &Context| {
                install_fonts(ctx);
                apply_theme(ctx, &theme);
            })
            .wgpu()
            .build_ui(app);
        Self { harness, theme }
    }

    /// A 1200×800 harness.
    pub fn wide(theme: Theme, app: impl FnMut(&mut Ui) + 'a) -> Self {
        Self::new(theme, Self::WIDE, app)
    }

    /// A 640×400 harness.
    pub fn compact(theme: Theme, app: impl FnMut(&mut Ui) + 'a) -> Self {
        Self::new(theme, Self::COMPACT, app)
    }

    /// The tokens the harness installed.
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Compare the current frame with `tests/snapshots/<name>.png`.
    ///
    /// Set `UPDATE_SNAPSHOTS=1` to write or update the committed image.
    pub fn snapshot(&mut self, name: impl Into<String>) {
        self.harness.snapshot(name);
    }
}

impl<'a> Deref for UiHarness<'a> {
    type Target = Harness<'a, ()>;

    fn deref(&self) -> &Self::Target {
        &self.harness
    }
}

impl DerefMut for UiHarness<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.harness
    }
}

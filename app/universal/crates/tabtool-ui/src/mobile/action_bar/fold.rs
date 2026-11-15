//! The fold-aware split of the contextual action bar.

use egui::Rect;

use crate::mobile::MOBILE_MIN_TOUCH_TARGET;

/// The space a vertical fold takes out of the bar.
///
/// A fold region is reported in screen coordinates; the action bar translates
/// it into its own rectangle before splitting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldRegion {
    /// The fold's leading edge in screen coordinates.
    pub x: f32,
    /// The fold's width.
    pub width: f32,
    /// The fold's height; a division region is taller than it is wide.
    pub height: f32,
}

impl FoldRegion {
    /// A screen-space fold at `x` with the given width and height.
    pub const fn new(x: f32, width: f32, height: f32) -> Self {
        Self { x, width, height }
    }

    /// Whether this is a vertical division region, not a horizontal one.
    pub fn is_vertical(&self) -> bool {
        self.height > self.width
    }
}

/// The gap a fold takes out of an action bar, with the split point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldGap {
    /// The width available for the leading actions, in bar coordinates.
    pub leading: f32,
    /// The unused fold width, in bar coordinates.
    pub width: f32,
    /// The width available for the trailing actions, in bar coordinates.
    pub trailing: f32,
    /// How many actions sit before the fold; `0` and `action_count` are valid
    /// when the whole bar fits on one side.
    pub split: usize,
}

impl FoldGap {
    /// Split `action_count` actions around `fold` in `bar_rect`.
    ///
    /// `fold` is a screen-space region; the bar translates it into its own
    /// coordinates. The bar only splits when the fold is active and clear of
    /// both edges. The preferred split is the centered iOS value, clamped so
    /// every visible action keeps [`MOBILE_MIN_TOUCH_TARGET`] points. If no
    /// split can keep all actions at the minimum, it returns `None` and the
    /// caller shows an unfolded bar instead of sub-44 pt buttons.
    pub fn make(fold: FoldRegion, bar_rect: Rect, action_count: usize) -> Option<Self> {
        Self::make_with_minimum(fold, bar_rect, action_count, MOBILE_MIN_TOUCH_TARGET)
    }

    /// [`FoldGap::make`] with an explicit minimum action width.
    ///
    /// The minimum is a parameter so tests can pin the narrowest target the
    /// layout must respect.
    pub fn make_with_minimum(
        fold: FoldRegion,
        bar_rect: Rect,
        action_count: usize,
        minimum_width: f32,
    ) -> Option<Self> {
        let bar_width = bar_rect.width();
        let leading = fold.x - bar_rect.left();
        if action_count <= 1
            || !fold.width.is_finite()
            || !leading.is_finite()
            || !bar_width.is_finite()
            || fold.width <= 0.0
            || leading <= 0.0
            || leading + fold.width >= bar_width
        {
            return None;
        }
        let minimum_width = if minimum_width.is_finite() && minimum_width > 0.0 {
            minimum_width
        } else {
            MOBILE_MIN_TOUCH_TARGET
        };
        let trailing = bar_width - (leading + fold.width);
        let max_leading = (leading / minimum_width).floor() as usize;
        let max_trailing = (trailing / minimum_width).floor() as usize;
        let min_split = action_count.saturating_sub(max_trailing);
        let max_split = max_leading.min(action_count);
        if min_split > max_split {
            return None;
        }
        Some(Self {
            leading,
            width: fold.width,
            trailing,
            split: action_count.div_ceil(2).clamp(min_split, max_split),
        })
    }
}

/// The split for a vertical fold, if the bar should split at all.
pub fn fold_gap(fold: Option<FoldRegion>, bar_rect: Rect, action_count: usize) -> Option<FoldGap> {
    fold.filter(FoldRegion::is_vertical)
        .and_then(|fold| FoldGap::make(fold, bar_rect, action_count))
}

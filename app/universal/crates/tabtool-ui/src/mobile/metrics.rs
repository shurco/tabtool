//! The touch-sized metrics layered on the shared [`crate::Theme`] tokens.
//!
//! Android uses this set on every screen size; the tablet keeps the phone
//! layout and only widens sheets. Safe-area and keyboard insets are supplied by
//! the platform and applied here, so every primitive lays out inside them.

use egui::{Rect, pos2};

/// The row heights the mobile grid offers, in points.
pub use tabtool_settings::{MOBILE_DEFAULT_ROW_HEIGHT, MOBILE_ROW_HEIGHTS};

/// The smallest touch target the interface allows, in points.
pub const MOBILE_MIN_TOUCH_TARGET: f32 = 44.0;

/// The mobile body text size, in points.
pub const MOBILE_BODY_TEXT: f32 = 17.0;

/// The tablet sheet width cap, in points.
pub const MOBILE_TABLET_SHEET_WIDTH: f32 = 540.0;

/// The window insets, in points.
///
/// The platform reports `[left, top, right, bottom]`, the order used by
/// [`Insets::from_array`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    /// The inset from the left edge.
    pub left: f32,
    /// The inset from the top edge.
    pub top: f32,
    /// The inset from the right edge.
    pub right: f32,
    /// The inset from the bottom edge.
    pub bottom: f32,
}

impl Insets {
    /// No insets.
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// Insets for each edge.
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Insets from the platform's `[left, top, right, bottom]` array.
    pub const fn from_array([left, top, right, bottom]: [f32; 4]) -> Self {
        Self::new(left, top, right, bottom)
    }

    /// The rectangle left inside `rect` after the insets.
    ///
    /// Negative insets count as zero and oversized insets collapse the
    /// rectangle to a point instead of inverting it.
    pub fn inner(self, rect: Rect) -> Rect {
        let left = self.left.max(0.0);
        let top = self.top.max(0.0);
        let right = self.right.max(0.0);
        let bottom = self.bottom.max(0.0);
        let min = pos2(rect.min.x + left, rect.min.y + top);
        let max = pos2(
            (rect.max.x - right).max(min.x),
            (rect.max.y - bottom).max(min.y),
        );
        Rect::from_min_max(min, max)
    }
}

/// The mobile geometry: touch targets, bars, sheets and current safe area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MobileMetrics {
    /// Body text, 17 pt.
    pub body: f32,
    /// Caption text, 11 pt.
    pub caption: f32,
    /// Navigation and sheet titles, 17 pt.
    pub title: f32,
    /// The grid row height, from the settings.
    pub row_height: f32,
    /// The height of a standard touch control, 44 pt.
    pub control_height: f32,
    /// The minimum touch target, 44 pt.
    pub min_touch_target: f32,
    /// The navigation bar height, 44 pt.
    pub toolbar_height: f32,
    /// The contextual action bar height, 44 pt.
    pub action_bar_height: f32,
    /// The keyboard accessory bar height, 44 pt.
    pub accessory_height: f32,
    /// The document status line height, 22 pt.
    pub status_line_height: f32,
    /// A two-line list row height, 44 pt.
    pub list_row_height: f32,
    /// The sheet content inset, 16 pt.
    pub sheet_padding: f32,
    /// The sheet header height, 44 pt.
    pub sheet_header_height: f32,
    /// The sheet width cap; `None` fills a phone.
    pub sheet_max_width: Option<f32>,
    /// The drag handle width, 36 pt.
    pub drag_handle_width: f32,
    /// The drag handle height, 5 pt.
    pub drag_handle_height: f32,
    /// The safe-area insets supplied by the platform.
    pub safe_area: Insets,
}

impl Default for MobileMetrics {
    fn default() -> Self {
        Self::phone()
    }
}

impl MobileMetrics {
    /// The phone layout: the sheet fills the width.
    pub fn phone() -> Self {
        Self {
            body: MOBILE_BODY_TEXT,
            caption: 11.0,
            title: MOBILE_BODY_TEXT,
            row_height: MOBILE_DEFAULT_ROW_HEIGHT,
            control_height: MOBILE_MIN_TOUCH_TARGET,
            min_touch_target: MOBILE_MIN_TOUCH_TARGET,
            toolbar_height: MOBILE_MIN_TOUCH_TARGET,
            action_bar_height: MOBILE_MIN_TOUCH_TARGET,
            accessory_height: MOBILE_MIN_TOUCH_TARGET,
            status_line_height: 22.0,
            list_row_height: MOBILE_MIN_TOUCH_TARGET,
            sheet_padding: 16.0,
            sheet_header_height: MOBILE_MIN_TOUCH_TARGET,
            sheet_max_width: None,
            drag_handle_width: 36.0,
            drag_handle_height: 5.0,
            safe_area: Insets::ZERO,
        }
    }

    /// The tablet layout: the same mobile layout with a wider sheet cap.
    pub fn tablet() -> Self {
        Self {
            sheet_max_width: Some(MOBILE_TABLET_SHEET_WIDTH),
            ..Self::phone()
        }
    }

    /// Use the stored row height, falling back to the default when unknown.
    pub fn with_row_height(mut self, stored: f32) -> Self {
        self.row_height = tabtool_settings::valid_mobile_row_height(stored);
        self
    }

    /// Apply the platform's safe-area insets.
    pub fn with_safe_area(mut self, safe_area: Insets) -> Self {
        self.safe_area = safe_area;
        self
    }

    /// The rectangle left inside `available` after the safe area.
    pub fn content_rect(&self, available: Rect) -> Rect {
        self.safe_area.inner(available)
    }

    /// The sheet width for `available_width`, including the sheet padding.
    ///
    /// A phone fills the width; a tablet caps it at
    /// [`MOBILE_TABLET_SHEET_WIDTH`] and never exceeds the window.
    pub fn sheet_width(&self, available_width: f32) -> f32 {
        self.sheet_max_width
            .map_or(available_width.max(0.0), |cap| {
                available_width.min(cap).max(0.0)
            })
    }

    /// The space the bottom edge takes: the larger of the keyboard and the
    /// safe area.
    pub fn bottom_inset(&self, keyboard_height: f32) -> f32 {
        self.safe_area.bottom.max(keyboard_height)
    }

    /// The tallest a sheet may be with `available_height` and the keyboard up.
    pub fn sheet_max_height(&self, available_height: f32, keyboard_height: f32) -> f32 {
        (available_height - self.safe_area.top - self.bottom_inset(keyboard_height)).max(0.0)
    }
}

//! Pure grid geometry: content size, pinned chrome, hit testing and the
//! visible ranges that drive virtualization.

use core::ops::Range;

use egui::{Pos2, Rect, Vec2, pos2, vec2};

/// The height of the pinned header row.
pub const HEADER_HEIGHT: f32 = 28.0;
/// The minimum width of the pinned row-number gutter.
pub const GUTTER_WIDTH: f32 = 40.0;
/// The default width of a desktop column before the user resizes it.
pub const DEFAULT_COLUMN_WIDTH: f32 = 100.0;
/// The narrowest a column may be resized to.
pub const MIN_COLUMN_WIDTH: f32 = 40.0;
/// How close to a header edge a resize may start.
pub const RESIZE_HANDLE_WIDTH: f32 = 4.0;
/// The width of the header's sort-arrow hit area.
pub const SORT_ARROW_WIDTH: f32 = 20.0;
/// Rows read above and below the viewport.
pub const ROW_OVERSCAN: usize = 1;
/// Columns read left and right of the viewport.
pub const COLUMN_OVERSCAN: usize = 1;

/// The approximate advance of one digit in the small monospaced gutter font.
const DIGIT_WIDTH: f32 = 7.0;
/// The horizontal padding inside the gutter, on both sides.
const GUTTER_PADDING: f32 = 12.0;

/// The width the row-number gutter needs for `row_count` displayed rows.
pub fn gutter_width(row_count: usize, header_offset: usize) -> f32 {
    let largest = row_count.saturating_add(header_offset);
    let digits = largest.to_string().len().max(2);
    (digits as f32 * DIGIT_WIDTH + GUTTER_PADDING).max(GUTTER_WIDTH)
}

/// Cell frames for the desktop grid.
///
/// Content coordinates start at the top-left of the scrollable content. The
/// header and gutter are positioned from the visible rect when they are drawn
/// pinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridGeometry<'a> {
    /// The width of every column, in visual order.
    pub widths: &'a [f32],
    /// The height of every data row.
    pub row_height: f32,
    /// The number of displayed data rows.
    pub row_count: usize,
    /// Whether the row-number gutter is visible.
    pub shows_row_numbers: bool,
    /// Physical rows hidden above the first displayed row (large mode headers).
    pub header_offset: usize,
}

impl<'a> GridGeometry<'a> {
    /// Builds geometry with no header offset.
    pub fn new(
        widths: &'a [f32],
        row_height: f32,
        row_count: usize,
        shows_row_numbers: bool,
    ) -> Self {
        Self {
            widths,
            row_height,
            row_count,
            shows_row_numbers,
            header_offset: 0,
        }
    }

    /// The number of columns.
    pub fn column_count(&self) -> usize {
        self.widths.len()
    }

    /// The width of the pinned gutter, or zero when it is hidden.
    pub fn gutter(&self) -> f32 {
        if self.shows_row_numbers {
            gutter_width(self.row_count, self.header_offset)
        } else {
            0.0
        }
    }

    /// The width of all columns together.
    pub fn columns_width(&self) -> f32 {
        self.widths.iter().copied().sum()
    }

    /// The full scrollable content size.
    pub fn content_size(&self) -> Vec2 {
        vec2(
            self.gutter() + self.columns_width(),
            HEADER_HEIGHT + self.row_count as f32 * self.row_height,
        )
    }

    /// The left edge of `column` in content coordinates.
    pub fn column_x(&self, column: usize) -> f32 {
        self.gutter() + self.widths.iter().take(column).copied().sum::<f32>()
    }

    /// The width of `column`.
    pub fn column_width(&self, column: usize) -> f32 {
        self.widths.get(column).copied().unwrap_or(0.0)
    }

    /// The frame of a data cell in content coordinates.
    pub fn cell_rect(&self, row: usize, column: usize) -> Rect {
        Rect::from_min_size(
            pos2(
                self.column_x(column),
                HEADER_HEIGHT + row as f32 * self.row_height,
            ),
            vec2(self.column_width(column), self.row_height),
        )
    }

    /// The header frame relative to the viewport top-left.
    ///
    /// `viewport` is in content coordinates. The header's y is pinned to the
    /// viewport top while x follows the horizontal scroll, so a caller adds the
    /// viewport's screen origin to place it.
    pub fn pinned_header_rect(&self, column: usize, viewport: Rect) -> Rect {
        Rect::from_min_size(
            pos2(self.column_x(column) - viewport.min.x, 0.0),
            vec2(self.column_width(column), HEADER_HEIGHT),
        )
    }

    /// The gutter frame relative to the viewport top-left.
    ///
    /// The gutter's x is pinned to the viewport left while y follows the
    /// vertical scroll.
    pub fn pinned_gutter_rect(&self, row: usize, viewport: Rect) -> Rect {
        Rect::from_min_size(
            pos2(
                0.0,
                HEADER_HEIGHT + row as f32 * self.row_height - viewport.min.y,
            ),
            vec2(self.gutter(), self.row_height),
        )
    }

    /// The pinned corner above the gutter, relative to the viewport top-left.
    pub fn pinned_corner_rect(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(self.gutter(), HEADER_HEIGHT))
    }

    /// The one-based display number shown beside display row `row`.
    ///
    /// Large mode hides physical header rows, so the offset is added to the
    /// display number. Filtered input likewise contributes display rows only:
    /// numbers are `1..visible row count`, not model row numbers.
    pub fn row_number(&self, row: usize) -> usize {
        row.saturating_add(self.header_offset).saturating_add(1)
    }

    /// The displayed rows touching the viewport, plus overscan.
    pub fn visible_rows(&self, visible: Rect) -> Range<usize> {
        if self.row_count == 0 || self.row_height <= 0.0 {
            return 0..0;
        }
        let first = (visible.min.y / self.row_height).floor().max(0.0) as usize;
        let after_last =
            ((visible.max.y - HEADER_HEIGHT).max(0.0) / self.row_height).ceil() as usize;
        let start = first.min(self.row_count);
        let end = after_last.clamp(start, self.row_count);
        let start = start.saturating_sub(ROW_OVERSCAN);
        let end = end.saturating_add(ROW_OVERSCAN).min(self.row_count);
        start..end
    }

    /// The displayed columns touching the viewport, plus overscan.
    pub fn visible_columns(&self, visible: Rect) -> Range<usize> {
        let gutter = self.gutter();
        let left = visible.min.x + gutter;
        let right = visible.max.x;
        let mut start = None;
        let mut end = 0;
        let mut x = gutter;
        for (index, width) in self.widths.iter().copied().enumerate() {
            let column_left = x;
            let column_right = x + width;
            if column_right > left && column_left < right {
                start.get_or_insert(index);
                end = index + 1;
            }
            if column_left >= right {
                break;
            }
            x = column_right;
        }
        let Some(start) = start else {
            return self.widths.len()..self.widths.len();
        };
        let start = start.saturating_sub(COLUMN_OVERSCAN);
        let end = end.saturating_add(COLUMN_OVERSCAN).min(self.widths.len());
        start..end
    }

    /// The data cell under a content-space point, if any.
    pub fn cell_at(&self, position: Pos2) -> Option<(usize, usize)> {
        if position.x < self.gutter() || position.y < HEADER_HEIGHT || self.row_height <= 0.0 {
            return None;
        }
        let row = ((position.y - HEADER_HEIGHT) / self.row_height).floor() as usize;
        if row >= self.row_count {
            return None;
        }
        let mut x = self.gutter();
        for (column, width) in self.widths.iter().copied().enumerate() {
            if position.x >= x && position.x < x + width {
                return Some((row, column));
            }
            x += width;
        }
        None
    }
}

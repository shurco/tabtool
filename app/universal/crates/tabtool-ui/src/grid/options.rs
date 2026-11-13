//! Presentation options: geometry, typography and menu capabilities.

use tabtool_core::SortDirection;
use tabtool_i18n::Language;

use super::context::GridMenuOptions;
use super::geometry::{DEFAULT_COLUMN_WIDTH, MIN_COLUMN_WIDTH};

/// The sort the header marks with an arrow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortSpec {
    /// The sorted visual column.
    pub column: usize,
    /// The sort direction.
    pub direction: SortDirection,
}

/// The user's grid presentation settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridAppearance {
    /// The height of every data row.
    pub row_height: f32,
    /// Whether odd rows use a tinted background.
    pub alternating_rows: bool,
    /// Whether the row-number gutter is visible.
    pub shows_row_numbers: bool,
}

impl Default for GridAppearance {
    fn default() -> Self {
        Self {
            row_height: 24.0,
            alternating_rows: false,
            shows_row_numbers: true,
        }
    }
}

/// The grid presentation settings.
#[derive(Clone, Debug, PartialEq)]
pub struct GridOptions {
    /// The height of every data row.
    pub row_height: f32,
    /// Whether odd rows use a tinted background.
    pub alternating_rows: bool,
    /// Whether the row-number gutter is visible.
    pub shows_row_numbers: bool,
    /// Physical rows hidden above display row zero (large-mode headers).
    ///
    /// This only shifts the row-number gutter. Cell and selection indexes stay
    /// display indexes because a large source adds its own offset inside
    /// `RowSource::cell`; adding it again here would double it.
    pub header_offset: usize,
    /// Whether cells can enter the editor.
    pub editable: bool,
    /// The document's decimal mark, used for numeric right alignment.
    pub decimal_mark: String,
    /// The width assigned to a column that has never been resized.
    pub default_column_width: f32,
    /// The narrowest a column may be resized to.
    pub min_column_width: f32,
    /// The content cap: the widest a column may be resized to. Infinite by
    /// default, matching the macOS grid.
    pub max_column_width: f32,
    /// The sort shown with an arrow in the header.
    pub sort: Option<SortSpec>,
    /// The language context menus render in.
    pub language: Language,
    /// The document capabilities the context menus depend on.
    pub menu: GridMenuOptions,
}

impl Default for GridOptions {
    fn default() -> Self {
        let appearance = GridAppearance::default();
        Self {
            row_height: appearance.row_height,
            alternating_rows: appearance.alternating_rows,
            shows_row_numbers: appearance.shows_row_numbers,
            header_offset: 0,
            editable: true,
            decimal_mark: ".".to_string(),
            default_column_width: DEFAULT_COLUMN_WIDTH,
            min_column_width: MIN_COLUMN_WIDTH,
            max_column_width: f32::INFINITY,
            sort: None,
            language: Language::System,
            menu: GridMenuOptions::default(),
        }
    }
}

impl GridOptions {
    /// Whether the grid draws horizontal separators.
    ///
    /// Alternating backgrounds replace them, matching the macOS appearance.
    pub fn shows_horizontal_lines(&self) -> bool {
        !self.alternating_rows
    }

    /// The physical row a display row maps to.
    pub fn physical_row(&self, display_row: usize) -> usize {
        display_row.saturating_add(self.header_offset)
    }
}

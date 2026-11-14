//! The selection-aware contextual action bar and its fold-aware split.

use egui::{Rect, Sense, Ui, Vec2, pos2};
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};

use super::{MOBILE_MIN_TOUCH_TARGET, MobileMetrics, TouchCell, bar_fill, hairline};
use crate::grid::GridSelection;

/// The buttons the contextual action bar can offer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ContextualActionId {
    /// Insert a row above the selected row.
    AddRowAbove,
    /// Insert a row below the selection.
    AddRowBelow,
    /// Insert a column left of the selected column.
    AddColumnLeft,
    /// Insert a column right of the selection.
    AddColumnRight,
    /// Delete the selected rows or columns.
    Delete,
    /// Sort the rows by the selected column, ascending.
    SortAscending,
    /// Sort the rows by the selected column, descending.
    SortDescending,
    /// Restore the pre-sort row order.
    RemoveSort,
    /// Rename the selected column.
    RenameColumn,
}

impl ContextualActionId {
    /// Every action, in display order.
    pub const ALL: [Self; 9] = [
        Self::AddRowAbove,
        Self::AddRowBelow,
        Self::AddColumnLeft,
        Self::AddColumnRight,
        Self::Delete,
        Self::SortAscending,
        Self::SortDescending,
        Self::RemoveSort,
        Self::RenameColumn,
    ];

    /// The catalog key for the accessible name.
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::AddRowAbove => "Add Row Above",
            Self::AddRowBelow => "Add Row Below",
            Self::AddColumnLeft => "Add Column Left",
            Self::AddColumnRight => "Add Column Right",
            Self::Delete => "Delete Selection",
            Self::SortAscending => "Sort Ascending",
            Self::SortDescending => "Sort Descending",
            Self::RemoveSort => "Remove Sort",
            Self::RenameColumn => "Rename Column",
        }
    }

    /// The symbol painted in the button.
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::AddRowAbove => "\u{2191}+",
            Self::AddRowBelow => "\u{2193}+",
            Self::AddColumnLeft => "\u{2190}+",
            Self::AddColumnRight => "\u{2192}+",
            Self::Delete => "\u{2212}",
            Self::SortAscending => "\u{2191}",
            Self::SortDescending => "\u{2193}",
            Self::RemoveSort => "\u{21BA}",
            Self::RenameColumn => "Aa",
        }
    }

    /// Whether the button is drawn in the destructive color.
    pub const fn destructive(self) -> bool {
        matches!(self, Self::Delete)
    }
}

/// One action and whether the document allows it now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextualAction {
    /// The action to run.
    pub id: ContextualActionId,
    /// Whether the action can run now.
    pub enabled: bool,
}

/// The actions a selection offers, or none without one.
///
/// A large document reports structural edits as unavailable, which removes the
/// bar entirely, matching the iOS `ContextualActions.ids`.
pub fn actions_for_selection(
    selection: &GridSelection,
    allows_structural_edits: bool,
) -> Vec<ContextualActionId> {
    if !allows_structural_edits {
        return Vec::new();
    }
    if !selection.rows.is_empty() {
        return vec![
            ContextualActionId::AddRowAbove,
            ContextualActionId::AddRowBelow,
            ContextualActionId::Delete,
        ];
    }
    if !selection.columns.is_empty() {
        return vec![
            ContextualActionId::AddColumnLeft,
            ContextualActionId::AddColumnRight,
            ContextualActionId::Delete,
            ContextualActionId::SortAscending,
            ContextualActionId::SortDescending,
            ContextualActionId::RemoveSort,
            ContextualActionId::RenameColumn,
        ];
    }
    Vec::new()
}

/// The action descriptors for the grid's live selection.
///
/// The grid state is the source of truth: the document's own selection is only
/// a mirror updated when a command is dispatched, so it can lag behind a tap.
pub fn actions_for(document: &Document, selection: &GridSelection) -> Vec<ContextualAction> {
    actions_for_selection(selection, !document.is_large_mode())
        .into_iter()
        .map(|id| ContextualAction {
            id,
            enabled: is_enabled(document, selection, id),
        })
        .collect()
}

/// Whether the document can run `id` for the live selection right now.
pub fn is_enabled(document: &Document, selection: &GridSelection, id: ContextualActionId) -> bool {
    let source = document.row_source();
    let column_count = source.map_or(0, |source| source.column_count());
    let row_count = source.map_or(0, |source| source.row_count());
    let selected_rows = selection.rows.iter().any(|row| *row < row_count);
    let selected_columns = selection
        .columns
        .iter()
        .any(|column| *column < column_count);

    match id {
        ContextualActionId::AddRowAbove => selected_rows && document.can_insert_rows(),
        ContextualActionId::AddRowBelow => document.can_insert_rows(),
        ContextualActionId::AddColumnLeft => selected_columns && document.can_edit_structure(),
        ContextualActionId::AddColumnRight => document.can_edit_structure(),
        ContextualActionId::Delete => {
            if selected_rows {
                document.can_insert_rows()
            } else {
                selected_columns && document.can_edit_structure() && column_count > 1
            }
        }
        ContextualActionId::SortAscending | ContextualActionId::SortDescending => {
            selected_columns && document.can_edit_structure()
        }
        ContextualActionId::RemoveSort => {
            document.active_sort().is_some() && document.can_edit_cells()
        }
        ContextualActionId::RenameColumn => selection
            .columns
            .iter()
            .next()
            .is_some_and(|column| document.can_rename_column(*column)),
    }
}

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

/// Draw the action bar; returns the action the user pressed.
///
/// `fold` is a screen-space region. The bar translates it into its own
/// coordinates before splitting and falls back to an unfolded bar when no
/// split can keep both sides at the minimum touch target.
pub fn contextual_action_bar(
    ui: &mut Ui,
    metrics: MobileMetrics,
    language: Language,
    actions: &[ContextualAction],
    fold: Option<FoldRegion>,
) -> Option<ContextualActionId> {
    if actions.is_empty() {
        return None;
    }
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(width, metrics.action_bar_height), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, bar_fill(ui));
    hairline(ui, rect, rect.top());

    let mut cells: Vec<(usize, Rect)> = Vec::with_capacity(actions.len());
    match fold_gap(fold, rect, actions.len()) {
        Some(gap) => {
            if gap.split > 0 {
                let leading_width = gap.leading / gap.split as f32;
                for (index, _) in actions.iter().enumerate().take(gap.split) {
                    cells.push((
                        index,
                        Rect::from_min_size(
                            pos2(rect.left() + index as f32 * leading_width, rect.top()),
                            Vec2::new(leading_width, rect.height()),
                        ),
                    ));
                }
            }
            if gap.split < actions.len() {
                let trailing_width = gap.trailing / (actions.len() - gap.split) as f32;
                for (index, _) in actions.iter().enumerate().skip(gap.split) {
                    cells.push((
                        index,
                        Rect::from_min_size(
                            pos2(
                                rect.left()
                                    + gap.leading
                                    + gap.width
                                    + (index - gap.split) as f32 * trailing_width,
                                rect.top(),
                            ),
                            Vec2::new(trailing_width, rect.height()),
                        ),
                    ));
                }
            }
        }
        None => {
            let inset = 4.0;
            let available = (rect.width() - 2.0 * inset).max(0.0);
            let cell_width = available / actions.len() as f32;
            for (index, _) in actions.iter().enumerate() {
                cells.push((
                    index,
                    Rect::from_min_size(
                        pos2(rect.left() + inset + index as f32 * cell_width, rect.top()),
                        Vec2::new(cell_width, rect.height()),
                    ),
                ));
            }
        }
    }

    let mut chosen = None;
    for (index, cell) in cells {
        let action = actions[index];
        let mut touch = TouchCell::new(action.id.glyph(), tr(language, action.id.title_key()))
            .enabled(action.enabled);
        if action.id.destructive() {
            touch = touch.destructive();
        }
        let response = touch.show(ui, cell, ("contextual-action", index));
        if response.clicked() {
            chosen = Some(action.id);
        }
    }
    chosen
}

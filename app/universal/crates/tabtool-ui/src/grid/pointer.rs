//! Pointer input for the grid.

use egui::{
    Context, Event, Id, Modifiers, PointerButton, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2,
};
use tabtool_core::RowSource;

use super::geometry::{GridGeometry, HEADER_HEIGHT, RESIZE_HANDLE_WIDTH};
use super::keyboard::cell_id;
use super::paint::{Canvas, VisibleCells};
use super::selection::{GridFocus, GridSelection};
use super::state::{GridOptions, GridState, header_for};

/// What a pointer gesture requested this frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct Interaction {
    cell: Option<(usize, usize, Modifiers)>,
    column: Option<(usize, Modifiers)>,
    row: Option<(usize, Modifiers)>,
    resize: Option<(usize, f32)>,
}

impl Interaction {
    /// Records a click on a data cell.
    pub fn cell(&mut self, click: (usize, usize, Modifiers)) {
        self.cell = Some(click);
    }

    /// Records a click on a column header.
    pub fn column(&mut self, column: usize, modifiers: Modifiers) {
        self.column = Some((column, modifiers));
    }

    /// Records a click on a row number.
    pub fn row(&mut self, row: usize, modifiers: Modifiers) {
        self.row = Some((row, modifiers));
    }

    /// Records a column drag.
    pub fn resize(&mut self, column: usize, delta: f32) {
        self.resize = Some((column, delta));
    }

    /// Applies the gesture to the persistent state.
    ///
    /// Selection clicks are applied after the editor has committed, and a
    /// click only starts a new edit when it is the second click on the focused
    /// cell and no editor was active at the start of the frame.
    pub fn apply(
        &mut self,
        ctx: &Context,
        base_id: Id,
        state: &mut GridState,
        source: &dyn RowSource,
        options: &GridOptions,
        was_editing: bool,
    ) {
        if let Some((column, delta)) = self.resize.take() {
            state.resize_column(column, delta, options);
        }
        let mut clicked = false;
        if let Some((row, column, modifiers)) = self.cell.take() {
            clicked = true;
            let already = !was_editing
                && state.focus == Some(GridFocus::new(row, column))
                && state.selection.rows.contains(&row);
            state
                .selection
                .select_row(row, modifiers.shift, modifiers.command, &mut state.anchor);
            state.focus = Some(GridFocus::new(row, column));
            if already && options.editable && !modifiers.shift && !modifiers.command {
                let text = source.cell(row, column);
                state.start_editing(row, column, text);
            }
        } else if let Some((column, modifiers)) = self.column.take() {
            clicked = true;
            state.selection.select_column(
                column,
                modifiers.shift,
                modifiers.command,
                &mut state.anchor,
            );
            state.focus = None;
            state.editing = None;
        } else if let Some((row, modifiers)) = self.row.take() {
            clicked = true;
            state
                .selection
                .select_row(row, modifiers.shift, modifiers.command, &mut state.anchor);
            let column = state.focus.map_or(0, |focus| {
                focus.column.min(source.column_count().saturating_sub(1))
            });
            state.focus = (source.column_count() > 0).then_some(GridFocus::new(row, column));
            state.editing = None;
        }
        if clicked
            && state.editing.is_none()
            && let Some(focus) = state.focus
        {
            ctx.memory_mut(|memory| {
                memory.request_focus(cell_id(base_id, focus.row, focus.column));
            });
        }
    }
}

/// Creates the click targets for the visible cells.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_cells(
    ui: &mut Ui,
    base_id: Id,
    visible: &VisibleCells,
    canvas: &Canvas,
    data: Rect,
    selection: &GridSelection,
    interaction: &mut Interaction,
) {
    for cell in &visible.cells {
        let screen = cell.rect.translate(canvas.origin.to_vec2());
        let interact = screen.intersect(data);
        if interact.width() <= 0.0 || interact.height() <= 0.0 {
            continue;
        }
        let response = ui.interact(
            interact,
            cell_id(base_id, cell.row, cell.column),
            Sense::click(),
        );
        let selected =
            selection.rows.contains(&cell.row) || selection.columns.contains(&cell.column);
        response
            .widget_info(|| WidgetInfo::selected(Role::Cell, true, selected, cell.text.clone()));
        if response.clicked() {
            interaction.cell((cell.row, cell.column, click_modifiers(ui.ctx())));
            response.request_focus();
        }
    }
}

/// Creates the click and resize targets for the pinned headers.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_headers(
    ui: &mut Ui,
    base_id: Id,
    headers: Option<&[String]>,
    geometry: &GridGeometry,
    canvas: &Canvas,
    selection: &GridSelection,
    interaction: &mut Interaction,
) {
    let clip = canvas.clip;
    let content_origin = canvas.origin;
    let viewport = canvas.viewport;
    let header_clip = Rect::from_min_max(
        pos2((clip.min.x + geometry.gutter()).min(clip.max.x), clip.min.y),
        pos2(clip.max.x, (clip.min.y + HEADER_HEIGHT).min(clip.max.y)),
    );
    for column in geometry.visible_columns(viewport) {
        let x = content_origin.x + geometry.column_x(column);
        let rect = Rect::from_min_size(
            pos2(x, clip.min.y),
            vec2(geometry.column_width(column), HEADER_HEIGHT),
        );
        let interact = rect.intersect(header_clip);
        if interact.width() <= 0.0 {
            continue;
        }
        let response = ui.interact(interact, base_id.with(("header", column)), Sense::click());
        let selected = selection.columns.contains(&column);
        response.widget_info(|| {
            WidgetInfo::selected(
                Role::ColumnHeader,
                true,
                selected,
                header_for(headers, column),
            )
        });
        if response.clicked() {
            interaction.column(column, click_modifiers(ui.ctx()));
        }

        let handle = Rect::from_min_max(
            pos2(rect.right() - RESIZE_HANDLE_WIDTH / 2.0, rect.min.y),
            pos2(rect.right() + RESIZE_HANDLE_WIDTH / 2.0, rect.max.y),
        )
        .intersect(clip);
        if handle.width() > 0.0 {
            let response = ui.interact(
                handle,
                base_id.with(("resize", column)),
                Sense::click_and_drag(),
            );
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if response.dragged() {
                interaction.resize(column, response.drag_delta().x);
            }
        }
    }
}

/// Creates the click targets for the pinned row-number gutter.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_gutter(
    ui: &mut Ui,
    base_id: Id,
    geometry: &GridGeometry,
    canvas: &Canvas,
    selection: &GridSelection,
    interaction: &mut Interaction,
) {
    let clip = canvas.clip;
    let content_origin = canvas.origin;
    let viewport = canvas.viewport;
    let gutter = geometry.gutter();
    if gutter <= 0.0 {
        return;
    }
    let gutter_clip = Rect::from_min_max(
        pos2(clip.min.x, (clip.min.y + HEADER_HEIGHT).min(clip.max.y)),
        pos2((clip.min.x + gutter).min(clip.max.x), clip.max.y),
    );
    for row in geometry.visible_rows(viewport) {
        let top = content_origin.y + HEADER_HEIGHT + row as f32 * geometry.row_height;
        let rect = Rect::from_min_max(
            pos2(clip.min.x, top),
            pos2(gutter_clip.max.x, top + geometry.row_height),
        );
        let interact = rect.intersect(gutter_clip);
        if interact.width() <= 0.0 || interact.height() <= 0.0 {
            continue;
        }
        let response = ui.interact(interact, base_id.with(("row-number", row)), Sense::click());
        let selected = selection.rows.contains(&row);
        response.widget_info(|| {
            WidgetInfo::selected(
                Role::RowHeader,
                true,
                selected,
                geometry.row_number(row).to_string(),
            )
        });
        if response.clicked() {
            interaction.row(row, click_modifiers(ui.ctx()));
        }
    }
}

fn click_modifiers(ctx: &Context) -> Modifiers {
    ctx.input(|input| {
        input
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers,
                    ..
                } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or(input.modifiers)
    })
}

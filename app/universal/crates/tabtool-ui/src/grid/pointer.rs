//! Pointer input, context menus and drags for the grid.

use egui::{Context, Event, Id, Modifiers, PointerButton, Rect, Role, Sense, Ui, WidgetInfo, pos2};

use super::context::{self, cell_menu, header_menu, row_menu};
use super::drag::{column_slot_at, row_slot_at};
use super::geometry::{GridGeometry, HEADER_HEIGHT, RESIZE_HANDLE_WIDTH};
use super::interaction::{ContextTap, Interaction};
use super::options::GridOptions;
use super::paint::{Canvas, VisibleCells};
use super::selection::GridElement;
use super::state::{GridState, header_for};
use super::touch::{GridTouch, TouchGesture};

/// Creates the click, context-menu and drag targets for the visible cells.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_cells(
    ui: &mut Ui,
    base_id: Id,
    visible: &VisibleCells,
    geometry: &GridGeometry,
    canvas: &Canvas,
    data: Rect,
    state: &GridState,
    options: &GridOptions,
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
            super::keyboard::cell_id(base_id, cell.row, cell.column),
            Sense::click_and_drag(),
        );
        if response.has_focus() {
            // The grid maps Tab, arrows, Home/End and pages itself; without
            // this filter egui would also move focus to a neighbouring widget
            // on the same key press.
            ui.ctx().memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    super::keyboard::cell_id(base_id, cell.row, cell.column),
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                );
            });
        }
        let selected = state.selection.rows.contains(&cell.row)
            || state.selection.columns.contains(&cell.column);
        response
            .widget_info(|| WidgetInfo::selected(Role::Cell, true, selected, cell.text.clone()));
        let long_touch = options.touch
            && response.long_touched()
            && GridTouch::new().long_press(GridElement::Cell) == TouchGesture::ContextMenu;
        if response.clicked() && !long_touch {
            interaction.cell((cell.row, cell.column, click_modifiers(ui.ctx())));
            response.request_focus();
        }
        if response.secondary_clicked() || long_touch {
            interaction.context(ContextTap {
                element: GridElement::Cell,
                row: cell.row,
                column: cell.column,
            });
        }
        if response.secondary_clicked() || long_touch || response.context_menu_opened() {
            interaction.note_menu_open();
            let selection = context::context_selection(
                GridElement::Cell,
                cell.row,
                cell.column,
                &state.selection,
            );
            let entries = cell_menu(
                &options.menu,
                &selection,
                geometry.row_count,
                geometry.column_count(),
            );
            if let Some(command) = context::show(&response, &entries, options.language, long_touch)
            {
                interaction.menu(command);
            }
        }
        if options.menu.row_structural && !options.touch && response.drag_started() {
            interaction.begin_row_drag(cell.row, cell.column);
        }
        if (response.drag_started() || response.dragged())
            && let Some(pointer) = ui.ctx().input(|input| input.pointer.interact_pos())
        {
            let content_y = canvas.viewport.min.y + pointer.y - canvas.clip.min.y;
            interaction.row_target(row_slot_at(geometry, content_y));
        }
        if response.drag_stopped() {
            let inside = ui
                .ctx()
                .input(|input| input.pointer.interact_pos())
                .is_some_and(|pointer| data.contains(pointer));
            interaction.end_row_drag(inside);
        }
    }
}

/// Creates the click, resize, context-menu and drag targets for the pinned
/// headers.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_headers(
    ui: &mut Ui,
    base_id: Id,
    headers: Option<&[String]>,
    geometry: &GridGeometry,
    canvas: &Canvas,
    state: &GridState,
    options: &GridOptions,
    interaction: &mut Interaction,
) {
    let clip = canvas.clip;
    let viewport = canvas.viewport;
    let header_clip = Rect::from_min_max(
        pos2((clip.min.x + geometry.gutter()).min(clip.max.x), clip.min.y),
        pos2(clip.max.x, (clip.min.y + HEADER_HEIGHT).min(clip.max.y)),
    );
    for column in geometry.visible_columns(viewport) {
        let rect = geometry
            .pinned_header_rect(column, viewport)
            .translate(clip.min.to_vec2());
        let interact = rect.intersect(header_clip);
        if interact.width() <= 0.0 {
            continue;
        }
        let response = ui.interact(
            interact,
            base_id.with(("header", column)),
            Sense::click_and_drag(),
        );
        let selected = state.selection.columns.contains(&column);
        response.widget_info(|| {
            WidgetInfo::selected(
                Role::ColumnHeader,
                true,
                selected,
                header_for(headers, column),
            )
        });
        let long_touch = options.touch
            && response.long_touched()
            && GridTouch::new().long_press(GridElement::ColumnHeader) == TouchGesture::ContextMenu;
        if response.clicked() && !long_touch {
            interaction.column(column, click_modifiers(ui.ctx()));
        }
        if response.secondary_clicked() || long_touch {
            interaction.context(ContextTap {
                element: GridElement::ColumnHeader,
                row: 0,
                column,
            });
        }
        if response.secondary_clicked() || long_touch || response.context_menu_opened() {
            interaction.note_menu_open();
            let selection =
                context::context_selection(GridElement::ColumnHeader, 0, column, &state.selection);
            let entries = header_menu(&options.menu, column, &selection, geometry.column_count());
            if let Some(command) = context::show(&response, &entries, options.language, long_touch)
            {
                interaction.menu(command);
            }
        }
        if !options.touch && response.drag_started() {
            // A press on the resize handle is a resize, not a header drag.
            let on_handle = ui.ctx().input(|input| {
                input
                    .pointer
                    .press_origin()
                    .is_some_and(|pointer| pointer.x >= rect.right() - RESIZE_HANDLE_WIDTH)
            });
            if !on_handle && options.menu.structural {
                interaction.begin_column_drag(column);
            }
        }
        if (response.drag_started() || response.dragged())
            && let Some(pointer) = ui.ctx().input(|input| input.pointer.interact_pos())
        {
            let content_x = canvas.viewport.min.x + pointer.x - canvas.clip.min.x;
            interaction.column_target(column_slot_at(geometry, content_x));
        }
        if response.drag_stopped() {
            let inside = ui
                .ctx()
                .input(|input| input.pointer.interact_pos())
                .is_some_and(|pointer| canvas.clip.contains(pointer));
            interaction.end_column_drag(inside);
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

/// Creates the click, context-menu and drag targets for the pinned row-number
/// gutter.
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_gutter(
    ui: &mut Ui,
    base_id: Id,
    geometry: &GridGeometry,
    canvas: &Canvas,
    state: &GridState,
    options: &GridOptions,
    interaction: &mut Interaction,
) {
    let clip = canvas.clip;
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
        let rect = geometry
            .pinned_gutter_rect(row, viewport)
            .translate(clip.min.to_vec2())
            .intersect(Rect::from_min_max(
                clip.min,
                pos2(gutter_clip.max.x, clip.max.y),
            ));
        let interact = rect.intersect(gutter_clip);
        if interact.width() <= 0.0 || interact.height() <= 0.0 {
            continue;
        }
        let response = ui.interact(
            interact,
            base_id.with(("row-number", row)),
            Sense::click_and_drag(),
        );
        let selected = state.selection.rows.contains(&row);
        response.widget_info(|| {
            WidgetInfo::selected(
                Role::RowHeader,
                true,
                selected,
                geometry.row_number(row).to_string(),
            )
        });
        let long_touch = options.touch
            && response.long_touched()
            && GridTouch::new().long_press(GridElement::RowNumber) == TouchGesture::ContextMenu;
        if response.clicked() && !long_touch {
            interaction.row(row, click_modifiers(ui.ctx()));
        }
        if response.secondary_clicked() || long_touch {
            interaction.context(ContextTap {
                element: GridElement::RowNumber,
                row,
                column: 0,
            });
        }
        if response.secondary_clicked() || long_touch || response.context_menu_opened() {
            interaction.note_menu_open();
            let selection =
                context::context_selection(GridElement::RowNumber, row, 0, &state.selection);
            let entries = row_menu(&options.menu, &selection);
            if let Some(command) = context::show(&response, &entries, options.language, long_touch)
            {
                interaction.menu(command);
            }
        }
        if options.menu.row_structural && !options.touch && response.drag_started() {
            interaction.begin_row_drag(row, 0);
        }
        if (response.drag_started() || response.dragged())
            && let Some(pointer) = ui.ctx().input(|input| input.pointer.interact_pos())
        {
            let content_y = canvas.viewport.min.y + pointer.y - canvas.clip.min.y;
            interaction.row_target(row_slot_at(geometry, content_y));
        }
        if response.drag_stopped() {
            let inside = ui
                .ctx()
                .input(|input| input.pointer.interact_pos())
                .is_some_and(|pointer| canvas.clip.contains(pointer));
            interaction.end_row_drag(inside);
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

//! Pointer input, context menus and drags for the grid.

mod headers;

use egui::{Context, Event, Id, Modifiers, PointerButton, Rect, Role, Sense, Ui, WidgetInfo, pos2};

use super::context::{self, cell_menu, row_menu};
use super::drag::row_slot_at;
use super::geometry::{GridGeometry, HEADER_HEIGHT};
use super::interaction::{ContextTap, Interaction};
use super::options::GridOptions;
use super::paint::{Canvas, VisibleCells};
use super::selection::GridElement;
use super::state::GridState;
use super::touch::{GridTouch, TouchGesture};

pub(crate) use headers::interact_headers;

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

pub(super) fn hovered_cell(
    ui: &Ui,
    clip: Rect,
    viewport: Rect,
    geometry: &GridGeometry,
) -> Option<(usize, usize)> {
    let pointer = ui.ctx().input(|input| input.pointer.hover_pos())?;
    if !clip.contains(pointer)
        || pointer.x < clip.min.x + geometry.gutter()
        || pointer.y < clip.min.y + HEADER_HEIGHT
    {
        return None;
    }
    let content = pos2(
        viewport.min.x + pointer.x - clip.min.x,
        viewport.min.y + pointer.y - clip.min.y,
    );
    geometry.cell_at(content)
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

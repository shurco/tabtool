//! Pointer targets for the pinned column headers and their resize handles.

use egui::{Id, Rect, Role, Sense, Ui, WidgetInfo, pos2};

use super::click_modifiers;
use crate::grid::context::{self, header_menu};
use crate::grid::drag::column_slot_at;
use crate::grid::geometry::{GridGeometry, HEADER_HEIGHT, RESIZE_HANDLE_WIDTH};
use crate::grid::interaction::{ContextTap, Interaction};
use crate::grid::options::GridOptions;
use crate::grid::paint::Canvas;
use crate::grid::selection::GridElement;
use crate::grid::state::{GridState, header_for};
use crate::grid::touch::{GridTouch, TouchGesture};

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

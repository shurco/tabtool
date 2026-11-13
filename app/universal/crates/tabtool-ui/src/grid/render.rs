//! The render pass: scroll area, visible-cell reads and action collection.

use super::chrome;
use super::editor;
use super::geometry::{GridGeometry, HEADER_HEIGHT};
use super::keyboard;
use super::paint::{self, VisibleCells};
use super::pointer::{self, Interaction};
use super::{GridAction, GridView};
use crate::theme::current;
use egui::{Align, Rect, Sense, Ui, pos2};

/// Renders the grid and returns the actions requested this frame.
pub(crate) fn show(view: GridView<'_>, ui: &mut Ui) -> Vec<GridAction> {
    let GridView {
        source,
        state,
        options,
    } = view;
    let ctx = ui.ctx().clone();
    let base_id = ui.make_persistent_id("tabtool-grid");
    let editor_id = base_id.with("editor");
    state.sync(source, &options);

    keyboard::note_ime(&ctx, state);
    let mut actions = Vec::new();
    let mut scroll_to = None;
    let was_editing = state.editing.is_some();
    let mut active_editor = state.editing.take();
    keyboard::handle_keys(
        &ctx,
        source,
        state,
        base_id,
        editor_id,
        options.editable,
        &mut active_editor,
        &mut actions,
        &mut scroll_to,
    );
    active_editor = state.editing.take().or(active_editor);
    if state.pending_editor_focus {
        if let Some(editing) = &active_editor {
            editor::focus_with_caret_end(&ctx, editor_id, editing.caret);
        }
        state.pending_editor_focus = false;
    }

    let focused = ctx.memory(|memory| memory.focused());
    let grid_focused = state
        .focus
        .is_some_and(|focus| focused == Some(keyboard::cell_id(base_id, focus.row, focus.column)));
    let mut interaction = Interaction::default();
    let mut editor_clicked = false;
    let mut editor_lost_focus = false;
    let mut editor_visible = false;

    let scroll_area = egui::ScrollArea::both()
        .id_salt(base_id.with("scroll"))
        .scroll_offset(state.scroll)
        .auto_shrink([false, false])
        .content_margin(0)
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            scroll_bar: true,
            drag: egui::containers::scroll_area::DragScroll::Never,
            mouse_wheel: true,
        });
    let output = scroll_area.show_viewport(ui, |ui, viewport| {
        let theme = current(ui.ctx());
        let clip = ui.clip_rect();
        let widths = state.ordered_widths(source);
        let geometry = GridGeometry {
            widths: &widths,
            row_height: options.row_height,
            row_count: source.row_count(),
            shows_row_numbers: options.shows_row_numbers,
            header_offset: options.header_offset,
        };
        state.viewport = viewport;
        let content_origin = ui.cursor().min;
        let content_size = geometry.content_size().max(viewport.size());
        ui.allocate_exact_size(content_size, Sense::hover());
        ui.painter()
            .rect_filled(clip, 0, theme.colors.content_background);

        let data = paint::data_clip(clip, &geometry);
        let canvas = paint::Canvas::new(ui.painter(), content_origin, clip, viewport);
        state.hovered = hovered_cell(ui, clip, viewport, &geometry);

        let headers = source.headers();
        let visible = VisibleCells::read(source, &geometry, viewport, &options.decimal_mark);
        paint::paint_data(&canvas, &geometry, &options, state, &visible, grid_focused);
        pointer::interact_cells(
            ui,
            base_id,
            &visible,
            &canvas,
            data,
            &state.selection,
            &mut interaction,
        );

        if let Some(editing) = active_editor.as_mut() {
            let rect = geometry
                .cell_rect(editing.row, editing.column)
                .translate(content_origin.to_vec2());
            if rect.intersects(data) {
                editor_visible = true;
                let name = super::state::header_for(headers.as_deref(), editing.column);
                let right_aligned = visible
                    .cells
                    .iter()
                    .find(|cell| cell.row == editing.row && cell.column == editing.column)
                    .is_some_and(|cell| cell.right_aligned);
                let response =
                    editor::show(ui, editor_id, rect, data, editing, &name, right_aligned);
                editor_clicked = response.clicked();
                editor_lost_focus = response.lost_focus();
            }
        }

        pointer::interact_headers(
            ui,
            base_id,
            headers.as_deref(),
            &geometry,
            &canvas,
            &state.selection,
            &mut interaction,
        );
        pointer::interact_gutter(
            ui,
            base_id,
            &geometry,
            &canvas,
            &state.selection,
            &mut interaction,
        );

        chrome::paint_header(&canvas, &geometry, &options, state, headers.as_deref());
        chrome::paint_gutter(&canvas, &geometry, &options, state);

        if let Some(focus) = scroll_to {
            let rect = geometry
                .cell_rect(focus.row, focus.column)
                .translate(content_origin.to_vec2());
            ui.scroll_to_rect(rect, Some(Align::Center));
        }
    });
    state.scroll = output.state.clamped_offset();

    let clicked_anywhere = ctx.input(|input| input.pointer.any_click());
    if let Some(editing) = active_editor {
        if !editor_visible || (clicked_anywhere && !editor_clicked) || editor_lost_focus {
            actions.push(editing.commit_action());
        } else {
            state.editing = Some(editing);
        }
    }
    interaction.apply(&ctx, base_id, state, source, &options, was_editing);
    actions
}

fn hovered_cell(
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

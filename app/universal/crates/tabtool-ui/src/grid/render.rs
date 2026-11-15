//! The render pass: scroll area, visible-cell reads and action collection.

use super::chrome;
use super::drag;
use super::editor;
use super::geometry::{GridGeometry, HEADER_HEIGHT};
use super::interaction::Interaction;
use super::keyboard;
use super::paint::{self, VisibleCells};
use super::pointer;
use super::{GridAction, GridEditRequest, GridView};
use crate::theme::current;
use egui::containers::scroll_area::DragScroll;
use egui::{Align, Rect, Sense, Ui, pos2};
use tabtool_i18n::tr;

/// Renders the grid and returns the actions requested this frame.
pub(crate) fn show(
    view: GridView<'_>,
    ui: &mut Ui,
    touch_request: &mut Option<GridEditRequest>,
) -> Vec<GridAction> {
    let GridView {
        source,
        state,
        options,
    } = view;
    let ctx = ui.ctx().clone();
    let base_id = ui.make_persistent_id("tabtool-grid");
    let editor_id = base_id.with("editor");
    let header_id = base_id.with("header-editor");
    state.sync(source, &options);

    keyboard::note_ime(&ctx, state);
    let mut actions = Vec::new();
    let mut scroll_to = None;
    let was_editing = state.editing.is_some();
    let mut active_editor = state.editing.take();
    if options.keys_enabled {
        keyboard::handle_keys(
            &ctx,
            source,
            state,
            base_id,
            editor_id,
            &options,
            &mut active_editor,
            &mut actions,
            &mut scroll_to,
        );
        active_editor = state.editing.take().or(active_editor);
    } else if let Some(editing) = active_editor.take() {
        // A modal opened while the editor was live; keep its text so the
        // owner can commit it once the modal closes.
        state.editing = Some(editing);
    }
    if state.pending_editor_focus {
        if let Some(editing) = &active_editor {
            editor::focus_with_caret_end(&ctx, editor_id, editing.caret);
        }
        state.pending_editor_focus = false;
    }
    if state.pending_header_focus {
        if let Some(editing) = &state.header_editing {
            editor::focus_with_caret_end(&ctx, header_id, editing.text.chars().count());
        }
        state.pending_header_focus = false;
    }

    let focused = ctx.memory(|memory| memory.focused());
    let grid_focused = state
        .focus
        .is_some_and(|focus| focused == Some(keyboard::cell_id(base_id, focus.row, focus.column)));
    let mut interaction = Interaction::default();
    let mut editor_clicked = false;
    let mut editor_lost_focus = false;
    let mut editor_visible = false;
    let mut header_clicked = false;
    let mut header_lost_focus = false;
    let mut header_editor_visible = false;
    let mut pending_visible = false;

    let scroll_area = egui::ScrollArea::both()
        .id_salt(base_id.with("scroll"))
        .scroll_offset(state.scroll)
        .auto_shrink([false, false])
        .content_margin(0)
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            scroll_bar: true,
            drag: if options.touch {
                DragScroll::Always
            } else {
                DragScroll::Never
            },
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
        state.hovered = if options.touch {
            None
        } else {
            pointer::hovered_cell(ui, clip, viewport, &geometry)
        };
        if let Some(focus) = state.pending_scroll.take() {
            let rect = geometry
                .cell_rect(focus.row, focus.column)
                .translate(content_origin.to_vec2());
            if !data.contains_rect(rect) {
                ui.scroll_to_rect(rect, Some(Align::Center));
            }
        }

        let headers = source.headers();
        let visible = VisibleCells::read(source, &geometry, viewport, &options.decimal_mark);
        if let Some(focus) = state.pending_focus
            && visible.rows.contains(&focus.row)
            && visible.columns.contains(&focus.column)
        {
            pending_visible = true;
        }
        paint::paint_data(&canvas, &geometry, &options, state, &visible, grid_focused);
        pointer::interact_cells(
            ui,
            base_id,
            &visible,
            &geometry,
            &canvas,
            data,
            state,
            &options,
            &mut interaction,
        );

        if let Some(editing) = active_editor.as_mut() {
            let rect = geometry
                .cell_rect(editing.row, editing.column)
                .translate(content_origin.to_vec2());
            if rect.intersects(data) {
                editor_visible = true;
                let header = super::state::header_for(headers.as_deref(), editing.column);
                let name = if header.is_empty() {
                    tr(options.language, "Edit Cell").to_owned()
                } else {
                    header
                };
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
            state,
            &options,
            &mut interaction,
        );
        pointer::interact_gutter(
            ui,
            base_id,
            &geometry,
            &canvas,
            state,
            &options,
            &mut interaction,
        );

        chrome::paint_header(&canvas, &geometry, &options, state, headers.as_deref());
        chrome::paint_gutter(&canvas, &geometry, &options, state);
        drag::paint_drop_indicator(&canvas, &geometry, state);

        if let Some(header_edit) = state.header_editing.as_mut() {
            let header_clip = Rect::from_min_max(
                pos2((clip.min.x + geometry.gutter()).min(clip.max.x), clip.min.y),
                pos2(clip.max.x, (clip.min.y + HEADER_HEIGHT).min(clip.max.y)),
            );
            if let Some(column) = source
                .column_ids()
                .iter()
                .position(|id| *id == header_edit.column)
            {
                let rect = geometry
                    .pinned_header_rect(column, viewport)
                    .translate(clip.min.to_vec2());
                if rect.intersects(header_clip) {
                    header_editor_visible = true;
                    let header = super::state::header_for(headers.as_deref(), column);
                    let name = if header.is_empty() {
                        tr(options.language, "Rename Column").to_owned()
                    } else {
                        header
                    };
                    let response =
                        editor::show_header(ui, header_id, rect, header_clip, header_edit, &name);
                    header_clicked = response.clicked();
                    header_lost_focus = response.lost_focus();
                }
            }
        }

        if let Some(focus) = scroll_to {
            let rect = geometry
                .cell_rect(focus.row, focus.column)
                .translate(content_origin.to_vec2());
            ui.scroll_to_rect(rect, Some(Align::Center));
        }
    });
    state.scroll = output.state.clamped_offset();

    let clicked_anywhere = ctx.input(|input| input.pointer.any_click());
    let drag_started = interaction.starts_drag();
    if let Some(editing) = active_editor {
        if drag_started
            || !editor_visible
            || (clicked_anywhere && !editor_clicked)
            || editor_lost_focus
        {
            actions.push(editing.commit_action());
            // A pointer commit never sends the IME Enter that would clear the
            // guard, so the next real Enter must start a fresh edit.
            state.ime_composing = false;
            state.ime_commit_guard = false;
        } else {
            state.editing = Some(editing);
        }
    }
    if state.header_editing.is_some()
        && (drag_started
            || !header_editor_visible
            || (clicked_anywhere && !header_clicked)
            || header_lost_focus)
    {
        editor::commit_header_editing(source, state, &mut actions);
    }
    if let Some(focus) = scroll_to {
        state.pending_focus = Some(focus);
    }
    interaction.apply(
        &ctx,
        base_id,
        state,
        source,
        &options,
        was_editing,
        &mut actions,
    );
    *touch_request = interaction.touch_request();
    // The target cell may have been outside the viewport when the key was
    // handled; keep requesting its focus until the scrolled viewport has
    // created it. egui drops focus for a widget it has not seen.
    match state.pending_focus {
        Some(focus) if state.focus == Some(focus) => {
            let cell = keyboard::cell_id(base_id, focus.row, focus.column);
            ctx.memory_mut(|memory| memory.request_focus(cell));
            if pending_visible && ctx.memory(|memory| memory.focused()) == Some(cell) {
                state.pending_focus = None;
            }
        }
        Some(_) => state.pending_focus = None,
        None => {}
    }
    state.grid_has_focus = state.focus.is_some_and(|focus| {
        ctx.memory(|memory| memory.focused())
            == Some(keyboard::cell_id(base_id, focus.row, focus.column))
    });
    state.menu_open = interaction.is_menu_open();
    actions
}

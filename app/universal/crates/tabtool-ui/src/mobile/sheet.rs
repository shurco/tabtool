//! The mobile sheet container: drag handle, title, safe area and scrolling.

use core::fmt;
use core::hash::Hash;

use egui::{
    Align, Align2, Button, Context, CornerRadius, Frame, Id, Layout, Margin, Modal, Rect, RichText,
    ScrollArea, Sense, Vec2, pos2,
};

use super::MobileMetrics;
use crate::fonts::heading_font;
use crate::theme::current;

/// The height of the row holding the drag handle.
const HANDLE_ROW_HEIGHT: f32 = 20.0;

/// How far the handle must travel down to dismiss the sheet.
const DRAG_CLOSE_THRESHOLD: f32 = 60.0;

/// The space above the title.
const TOP_PADDING: f32 = 8.0;

/// The strings and insets a sheet needs.
pub struct SheetOptions<'a> {
    /// The localized sheet title.
    pub title: &'a str,
    /// The localized close button label.
    pub close_label: &'a str,
    /// The platform keyboard height, or zero when it is hidden.
    pub keyboard_height: f32,
}

impl<'a> SheetOptions<'a> {
    /// A sheet titled `title` with a close button.
    pub fn new(title: &'a str, close_label: &'a str) -> Self {
        Self {
            title,
            close_label,
            keyboard_height: 0.0,
        }
    }

    /// Lift the sheet above the keyboard.
    pub fn keyboard_height(mut self, keyboard_height: f32) -> Self {
        self.keyboard_height = keyboard_height;
        self
    }
}

/// The result of showing a sheet.
pub struct SheetResponse<R> {
    /// What the sheet content returned.
    pub inner: R,
    /// Whether the close button, Escape or a drag asked to dismiss the sheet.
    pub close_requested: bool,
    /// The outer sheet width, including its padding.
    pub width: f32,
    /// The tallest the sheet was allowed to be.
    pub max_height: f32,
}

/// Show a bottom sheet over the screen.
///
/// The sheet sits above the safe-area bottom or the keyboard, whichever is
/// larger, and its content scrolls inside the remaining height. A drag on the
/// handle past 60 points, the close button, Escape or a backdrop click all
/// report [`SheetResponse::close_requested`].
pub fn sheet<R>(
    ctx: &Context,
    id_salt: impl Hash + fmt::Debug,
    metrics: MobileMetrics,
    options: SheetOptions<'_>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> SheetResponse<R> {
    let theme = current(ctx);
    let colors = theme.colors;
    let available = ctx.content_rect();
    let content_rect = metrics.safe_area.inner(available);
    let width = metrics.sheet_width(content_rect.width());
    let max_height = metrics.sheet_max_height(available.height(), options.keyboard_height);
    let bottom_offset = metrics.bottom_inset(options.keyboard_height);
    let padding = metrics.sheet_padding;
    let content_width = (width - 2.0 * padding).max(1.0);
    let vertical_padding = TOP_PADDING + padding;
    let content_max_height = (max_height - vertical_padding).max(metrics.control_height);

    let style = ctx.style_of(ctx.theme());
    let frame = Frame::popup(style.as_ref())
        .fill(colors.control_background)
        .stroke(egui::Stroke::NONE)
        .corner_radius(CornerRadius {
            nw: 16,
            ne: 16,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin {
            left: padding as i8,
            right: padding as i8,
            top: TOP_PADDING as i8,
            bottom: padding as i8,
        });

    let id = Id::unique(id_salt);
    let drag_id = id.with("mobile-sheet-drag");
    let horizontal_offset = content_rect.center().x - available.center().x;
    let area = Modal::default_area(id).anchor(
        Align2::CENTER_BOTTOM,
        Vec2::new(horizontal_offset, -bottom_offset),
    );
    let mut close_requested = false;

    let response = Modal::new(id).area(area).frame(frame).show(ctx, |ui| {
        ui.set_width(content_width);
        ui.set_max_height(content_max_height);

        let (handle_rect, handle) =
            ui.allocate_exact_size(Vec2::new(content_width, HANDLE_ROW_HEIGHT), Sense::drag());
        let handle_center = pos2(handle_rect.center().x, handle_rect.center().y);
        ui.painter().rect_filled(
            Rect::from_center_size(
                handle_center,
                Vec2::new(metrics.drag_handle_width, metrics.drag_handle_height),
            ),
            CornerRadius::same(metrics.drag_handle_height as u8),
            colors.tertiary_label,
        );
        let mut drag_distance = ctx
            .data(|data| data.get_temp::<f32>(drag_id))
            .unwrap_or(0.0);
        if handle.dragged() {
            drag_distance = handle
                .total_drag_delta()
                .map_or(drag_distance, |delta| delta.y);
            ctx.data_mut(|data| data.insert_temp(drag_id, drag_distance));
        }
        if handle.drag_stopped() {
            close_requested = drag_distance > DRAG_CLOSE_THRESHOLD;
            ctx.data_mut(|data| data.insert_temp(drag_id, 0.0));
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(options.title)
                    .font(heading_font(metrics.title))
                    .color(colors.label),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let button = Button::new(RichText::new(options.close_label).size(metrics.body))
                    .min_size(Vec2::new(88.0, metrics.control_height));
                if ui.add(button).clicked() {
                    close_requested = true;
                }
            });
        });
        ui.add_space(8.0);

        let scroll_height =
            (content_max_height - 20.0 - metrics.sheet_header_height).max(metrics.control_height);
        ScrollArea::vertical()
            .max_height(scroll_height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.set_width(content_width);
                add_contents(ui)
            })
            .inner
    });

    if response.should_close() {
        close_requested = true;
    }
    SheetResponse {
        inner: response.inner,
        close_requested,
        width,
        max_height,
    }
}

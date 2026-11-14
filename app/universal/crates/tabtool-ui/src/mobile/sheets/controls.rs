//! Touch-sized buttons, checkboxes, value fields and pickers.

use core::fmt;
use core::hash::Hash;

use egui::{
    Align2, CornerRadius, FontId, Response, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo, pos2,
};

use super::layout::SheetStyle;
use crate::mobile::MobileMetrics;
use crate::theme::current;
use crate::widgets::{ButtonKind, Picker, PushButton};

/// A labelled checkbox on one full-width touch row.
pub(crate) fn touch_toggle(
    ui: &mut Ui,
    style: SheetStyle,
    label: &str,
    on: &mut bool,
    enabled: bool,
) -> Response {
    let theme = current(ui.ctx());
    let width = ui.available_width();
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::new(width, style.metrics.control_height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.clicked() && enabled {
        *on = !*on;
        response.mark_changed();
    }

    let text_color = if enabled {
        theme.colors.label
    } else {
        theme.colors.tertiary_label
    };
    ui.painter().text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(style.metrics.body),
        text_color,
    );
    let box_rect = checkbox_rect(rect, style.metrics);
    paint_checkbox(ui, box_rect, *on, enabled);

    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, enabled, *on, label));
    response.accessible_name(label)
}

/// A checkbox without a visible label, sized for a criterion card header.
pub(crate) fn touch_checkbox(
    ui: &mut Ui,
    style: SheetStyle,
    name: &str,
    on: &mut bool,
    enabled: bool,
) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::splat(style.metrics.control_height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.clicked() && enabled {
        *on = !*on;
        response.mark_changed();
    }
    paint_checkbox(ui, checkbox_rect(rect, style.metrics), *on, enabled);
    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, enabled, *on, name));
    response.accessible_name(name)
}

/// The value field the native text editor opens over.
///
/// The sheet never draws an in-window text field: a tap returns the response
/// and the caller asks the platform editor to open.
pub(crate) fn touch_field(
    ui: &mut Ui,
    style: SheetStyle,
    name: &str,
    value: &str,
    hint: &str,
    enabled: bool,
) -> Response {
    let theme = current(ui.ctx());
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width, style.metrics.control_height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let radius = CornerRadius::same(theme.metrics.corner_radius);
    ui.painter().rect(
        rect,
        radius,
        theme.colors.control_background,
        Stroke::new(1.0, theme.colors.control_border),
        StrokeKind::Inside,
    );
    let (text, color) = if value.is_empty() {
        (hint, theme.colors.tertiary_label)
    } else if enabled {
        (value, theme.colors.label)
    } else {
        (value, theme.colors.tertiary_label)
    };
    ui.painter().with_clip_rect(rect.shrink(6.0)).text(
        pos2(rect.left() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(style.metrics.body),
        color,
    );
    response.widget_info(|| WidgetInfo::text_edit(enabled, value, value, hint));
    response.accessible_name(name)
}

/// The enablement and width of a touch picker.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PickerTouch {
    /// Whether the picker accepts touches.
    pub enabled: bool,
    /// The button width, or the available width when `None`.
    pub width: Option<f32>,
}

impl PickerTouch {
    /// An enabled picker filling the available width.
    pub(crate) fn enabled(enabled: bool) -> Self {
        Self {
            enabled,
            width: None,
        }
    }

    /// Use an explicit width.
    pub(crate) fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

/// A touch-sized picker filling the available width.
pub(crate) fn touch_picker<'a>(
    ui: &mut Ui,
    style: SheetStyle,
    id_salt: impl Hash + fmt::Debug,
    name: &'a str,
    selected: &'a str,
    options: &'a [&'a str],
    enabled: bool,
) -> Option<usize> {
    touch_picker_sized(
        ui,
        style,
        id_salt,
        name,
        selected,
        options,
        PickerTouch::enabled(enabled),
    )
}

/// A touch-sized picker with an explicit width.
pub(crate) fn touch_picker_sized<'a>(
    ui: &mut Ui,
    style: SheetStyle,
    id_salt: impl Hash + fmt::Debug,
    name: &'a str,
    selected: &'a str,
    options: &'a [&'a str],
    touch: PickerTouch,
) -> Option<usize> {
    let width = touch.width.unwrap_or_else(|| ui.available_width());
    Picker::new(id_salt, name, selected, options)
        .width(width)
        .height(style.metrics.control_height)
        .text_size(style.metrics.body)
        .enabled(touch.enabled)
        .show(ui)
}

/// A bordered or prominent button with a full touch height.
pub(crate) fn touch_button<'a>(
    style: SheetStyle,
    label: &'a str,
    kind: ButtonKind,
) -> PushButton<'a> {
    PushButton::new(label, kind)
        .height(style.metrics.control_height)
        .text_size(style.metrics.body)
        .min_width(style.metrics.min_touch_target)
}

/// A full-width action button.
pub(crate) fn touch_wide_button(
    ui: &mut Ui,
    style: SheetStyle,
    label: &str,
    kind: ButtonKind,
) -> Response {
    let width = ui.available_width();
    touch_button(style, label, kind).min_width(width).show(ui)
}

fn checkbox_rect(rect: egui::Rect, metrics: MobileMetrics) -> egui::Rect {
    let size = 22.0_f32.min(metrics.control_height - 8.0).max(16.0);
    egui::Rect::from_center_size(
        pos2(rect.right() - size / 2.0 - 10.0, rect.center().y),
        Vec2::splat(size),
    )
}

fn paint_checkbox(ui: &Ui, rect: egui::Rect, on: bool, enabled: bool) {
    let theme = current(ui.ctx());
    let radius = CornerRadius::same(theme.metrics.small_corner_radius);
    let painter = ui.painter();
    if on {
        let fill = if enabled {
            theme.colors.accent
        } else {
            theme.colors.tertiary_label
        };
        painter.rect_filled(rect, radius, fill);
        let check = Stroke::new(1.8, theme.colors.control_background);
        let points = [
            rect.center() + Vec2::new(-5.0, 0.0),
            rect.center() + Vec2::new(-1.5, 3.5),
            rect.center() + Vec2::new(5.0, -3.5),
        ];
        painter.add(egui::Shape::line(vec![points[0], points[1]], check));
        painter.add(egui::Shape::line(vec![points[1], points[2]], check));
    } else {
        painter.rect(
            rect,
            radius,
            theme.colors.control_background,
            Stroke::new(1.0, theme.colors.control_border),
            StrokeKind::Inside,
        );
    }
}

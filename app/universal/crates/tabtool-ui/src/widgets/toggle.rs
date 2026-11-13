//! The macOS-style checkbox used by the format bar and settings.

use egui::{
    Align2, CornerRadius, FontId, Key, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    Vec2, Widget, WidgetInfo,
};

use crate::theme::current;

/// A labelled checkbox.
pub fn toggle(ui: &mut Ui, label: &str, on: &mut bool) -> Response {
    ui.add(Toggle { label, on })
}

/// A labelled checkbox that can be disabled.
pub fn toggle_enabled(ui: &mut Ui, label: &str, on: &mut bool, enabled: bool) -> Response {
    ui.add_enabled(enabled, Toggle { label, on })
}

struct Toggle<'a> {
    label: &'a str,
    on: &'a mut bool,
}

impl Widget for Toggle<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = current(ui.ctx());
        let enabled = ui.is_enabled();
        let text_color = if enabled {
            theme.colors.label
        } else {
            theme.colors.tertiary_label
        };
        let font = FontId::proportional(theme.typography.body);
        let galley = ui
            .painter()
            .layout_no_wrap(self.label.to_owned(), font.clone(), text_color);
        let box_size = 13.0;
        let gap = 6.0;
        let height = theme.metrics.control_height.max(galley.size().y);
        let (rect, mut response) = ui.allocate_exact_size(
            Vec2::new(box_size + gap + galley.size().x, height),
            Sense::click(),
        );

        let toggled_by_key = response.has_focus() && ui.input(|i| i.key_pressed(Key::Space));
        if response.clicked() || toggled_by_key {
            *self.on = !*self.on;
            response.mark_changed();
        }

        let box_rect = Rect::from_center_size(
            Pos2::new(rect.left() + box_size / 2.0, rect.center().y),
            Vec2::splat(box_size),
        );
        let radius = CornerRadius::same(theme.metrics.small_corner_radius);
        let painter = ui.painter();
        if *self.on {
            painter.rect_filled(box_rect, radius, theme.colors.accent);
            let check = Stroke::new(1.6, theme.colors.control_background);
            let points = [
                box_rect.center() + Vec2::new(-3.5, 0.0),
                box_rect.center() + Vec2::new(-1.0, 2.5),
                box_rect.center() + Vec2::new(3.5, -2.5),
            ];
            painter.add(Shape::line(vec![points[0], points[1]], check));
            painter.add(Shape::line(vec![points[1], points[2]], check));
        } else {
            painter.rect(
                box_rect,
                radius,
                theme.colors.control_background,
                Stroke::new(1.0, theme.colors.control_border),
                StrokeKind::Inside,
            );
        }
        painter.text(
            Pos2::new(box_rect.right() + gap, rect.center().y),
            Align2::LEFT_CENTER,
            self.label,
            font,
            text_color,
        );

        response.widget_info(|| {
            WidgetInfo::selected(egui::Role::CheckBox, enabled, *self.on, self.label)
        });
        let response = response.accessible_name(self.label);

        if response.has_focus() {
            ui.painter().rect_stroke(
                box_rect.expand(2.0),
                CornerRadius::same(theme.metrics.small_corner_radius + 2),
                Stroke::new(theme.metrics.focus_ring_width, theme.colors.focus_ring),
                StrokeKind::Outside,
            );
        }
        response
    }
}

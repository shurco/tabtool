//! The shared line and polygon builders behind the icons.

use egui::{Pos2, Rect, Shape, Stroke, pos2};

/// The direction of a straight arrow icon.
#[derive(Clone, Copy)]
pub(super) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub(super) fn arrow_shapes(rect: Rect, direction: Direction, stroke: Stroke) -> Vec<Shape> {
    let center = rect.center();
    let (shaft, head) = match direction {
        Direction::Up => (
            [pos2(center.x, rect.bottom()), pos2(center.x, rect.top())],
            [
                pos2(
                    rect.left() + rect.width() * 0.22,
                    rect.top() + rect.height() * 0.3,
                ),
                pos2(center.x, rect.top()),
                pos2(
                    rect.right() - rect.width() * 0.22,
                    rect.top() + rect.height() * 0.3,
                ),
            ],
        ),
        Direction::Down => (
            [pos2(center.x, rect.top()), pos2(center.x, rect.bottom())],
            [
                pos2(
                    rect.left() + rect.width() * 0.22,
                    rect.bottom() - rect.height() * 0.3,
                ),
                pos2(center.x, rect.bottom()),
                pos2(
                    rect.right() - rect.width() * 0.22,
                    rect.bottom() - rect.height() * 0.3,
                ),
            ],
        ),
        Direction::Left => (
            [pos2(rect.right(), center.y), pos2(rect.left(), center.y)],
            [
                pos2(
                    rect.left() + rect.width() * 0.3,
                    rect.top() + rect.height() * 0.22,
                ),
                pos2(rect.left(), center.y),
                pos2(
                    rect.left() + rect.width() * 0.3,
                    rect.bottom() - rect.height() * 0.22,
                ),
            ],
        ),
        Direction::Right => (
            [pos2(rect.left(), center.y), pos2(rect.right(), center.y)],
            [
                pos2(
                    rect.right() - rect.width() * 0.3,
                    rect.top() + rect.height() * 0.22,
                ),
                pos2(rect.right(), center.y),
                pos2(
                    rect.right() - rect.width() * 0.3,
                    rect.bottom() - rect.height() * 0.22,
                ),
            ],
        ),
    };
    vec![
        Shape::line_segment(shaft, stroke),
        Shape::line(head.to_vec(), stroke),
    ]
}

pub(super) fn add_shapes(rect: Rect, direction: Direction, stroke: Stroke) -> Vec<Shape> {
    let arrow_rect = match direction {
        Direction::Up | Direction::Down => Rect::from_min_max(
            rect.min,
            pos2(rect.right() - rect.width() * 0.3, rect.bottom()),
        ),
        Direction::Left | Direction::Right => Rect::from_min_max(
            rect.min,
            pos2(rect.right(), rect.bottom() - rect.height() * 0.3),
        ),
    };
    let mut shapes = arrow_shapes(arrow_rect, direction, stroke);
    shapes.extend(plus_shapes(
        pos2(
            rect.right() - rect.width() * 0.14,
            rect.bottom() - rect.height() * 0.14,
        ),
        rect.width() * 0.16,
        stroke,
    ));
    shapes
}

pub(super) fn plus_shapes(center: Pos2, arm: f32, stroke: Stroke) -> Vec<Shape> {
    vec![
        Shape::line_segment(
            [
                pos2(center.x - arm, center.y),
                pos2(center.x + arm, center.y),
            ],
            stroke,
        ),
        Shape::line_segment(
            [
                pos2(center.x, center.y - arm),
                pos2(center.x, center.y + arm),
            ],
            stroke,
        ),
    ]
}

/// A curved undo/redo arrow with an arrowhead on the leading end.
pub(super) fn arc_arrow(
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    stroke: Stroke,
    mirrored: bool,
) -> Vec<Shape> {
    let center = pos2((left + right) * 0.5, bottom - (bottom - top) * 0.3);
    let radius = (right - left) * 0.42;
    let mut points: Vec<Pos2> = (0..=8)
        .map(|step| {
            let angle = std::f32::consts::PI + step as f32 * std::f32::consts::PI / 8.0;
            pos2(
                center.x + angle.cos() * radius,
                center.y + angle.sin() * radius,
            )
        })
        .collect();
    if mirrored {
        points = points
            .into_iter()
            .map(|point| pos2(2.0 * center.x - point.x, point.y))
            .collect();
    }
    let head = points[0];
    let inward = if mirrored { -1.0 } else { 1.0 };
    vec![
        Shape::line(points, stroke),
        Shape::line(
            vec![
                pos2(
                    head.x + inward * (right - left) * 0.22,
                    head.y - (bottom - top) * 0.16,
                ),
                head,
                pos2(
                    head.x + inward * (right - left) * 0.28,
                    head.y + (bottom - top) * 0.14,
                ),
            ],
            stroke,
        ),
    ]
}

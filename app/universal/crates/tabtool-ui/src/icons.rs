//! Deterministic vector icons for touch controls.
//!
//! The mobile bars used to draw Unicode symbols. Android's installed system
//! fonts do not cover the arrows and triangles, so those controls rendered as
//! tofu boxes. These icons are painted from line and polygon shapes, so they
//! look the same with any font and need no bundled font file.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// One toolbar, accessory or disclosure icon.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Icon {
    /// Back to the documents library.
    Back,
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// Filter.
    Filter,
    /// Export.
    Export,
    /// The overflow menu.
    Overflow,
    /// Previous cell.
    Previous,
    /// Next cell.
    Next,
    /// Commit the edit.
    Done,
    /// A list row's disclosure chevron.
    Chevron,
    /// App settings.
    Settings,
    /// Add a document or criterion.
    Plus,
    /// Delete or remove.
    Minus,
    /// Upward sort or insertion.
    ArrowUp,
    /// Downward sort or insertion.
    ArrowDown,
    /// Leftward insertion.
    ArrowLeft,
    /// Rightward insertion.
    ArrowRight,
    /// Insert a row above.
    AddRowAbove,
    /// Insert a row below.
    AddRowBelow,
    /// Insert a column left.
    AddColumnLeft,
    /// Insert a column right.
    AddColumnRight,
}

impl Icon {
    /// Every icon, for coverage tests.
    pub const ALL: [Self; 21] = [
        Self::Back,
        Self::Undo,
        Self::Redo,
        Self::Filter,
        Self::Export,
        Self::Overflow,
        Self::Previous,
        Self::Next,
        Self::Done,
        Self::Chevron,
        Self::Settings,
        Self::Plus,
        Self::Minus,
        Self::ArrowUp,
        Self::ArrowDown,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::AddRowAbove,
        Self::AddRowBelow,
        Self::AddColumnLeft,
        Self::AddColumnRight,
    ];
}

/// The shapes one icon draws inside `rect`.
pub fn icon_shapes(icon: Icon, rect: Rect, color: Color32) -> Vec<Shape> {
    let side = rect.width().min(rect.height());
    if !side.is_finite() || side <= 0.0 {
        return Vec::new();
    }
    let stroke = Stroke::new((side * 0.075).clamp(1.5, 2.6), color);
    let square = Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
    let inner = square.shrink(side * 0.28);
    let left = inner.left();
    let right = inner.right();
    let top = inner.top();
    let bottom = inner.bottom();
    let center = inner.center();
    let width = inner.width();
    let height = inner.height();

    match icon {
        Icon::Back | Icon::Previous => vec![Shape::line(
            vec![pos2(right, top), pos2(left, center.y), pos2(right, bottom)],
            stroke,
        )],
        Icon::Next | Icon::Chevron => vec![Shape::line(
            vec![pos2(left, top), pos2(right, center.y), pos2(left, bottom)],
            stroke,
        )],
        Icon::Undo => arc_arrow(left, right, top, bottom, stroke, false),
        Icon::Redo => arc_arrow(left, right, top, bottom, stroke, true),
        Icon::Filter => vec![Shape::closed_line(
            vec![
                pos2(left, top),
                pos2(right, top),
                pos2(center.x + width * 0.12, center.y),
                pos2(center.x + width * 0.12, bottom),
                pos2(center.x - width * 0.12, bottom - height * 0.1),
                pos2(center.x - width * 0.12, center.y),
            ],
            stroke,
        )],
        Icon::Export => {
            let tip = pos2(right, top);
            vec![
                Shape::line(vec![pos2(left, bottom), tip], stroke),
                Shape::line(
                    vec![
                        pos2(right - width * 0.42, top),
                        tip,
                        pos2(right, top + height * 0.42),
                    ],
                    stroke,
                ),
                Shape::line_segment([pos2(left, bottom), pos2(center.x, bottom)], stroke),
            ]
        }
        Icon::Overflow => {
            let radius = side * 0.045;
            let y = center.y;
            vec![
                Shape::circle_filled(pos2(center.x - width * 0.34, y), radius, color),
                Shape::circle_filled(center, radius, color),
                Shape::circle_filled(pos2(center.x + width * 0.34, y), radius, color),
            ]
        }
        Icon::Done => vec![Shape::line(
            vec![
                pos2(left, center.y),
                pos2(center.x - width * 0.08, bottom),
                pos2(right, top),
            ],
            stroke,
        )],
        Icon::Settings => {
            let mut shapes = Vec::with_capacity(6);
            let rows = [0.2, 0.5, 0.8];
            let knobs = [0.32, 0.68, 0.44];
            for (row, knob) in rows.into_iter().zip(knobs) {
                let y = top + height * row;
                shapes.push(Shape::line_segment([pos2(left, y), pos2(right, y)], stroke));
                shapes.push(Shape::circle_filled(
                    pos2(left + width * knob, y),
                    side * 0.075,
                    color,
                ));
            }
            shapes
        }
        Icon::Plus => plus_shapes(center, side * 0.2, stroke),
        Icon::Minus => vec![Shape::line_segment(
            [pos2(left, center.y), pos2(right, center.y)],
            stroke,
        )],
        Icon::ArrowUp => arrow_shapes(inner, Direction::Up, stroke),
        Icon::ArrowDown => arrow_shapes(inner, Direction::Down, stroke),
        Icon::ArrowLeft => arrow_shapes(inner, Direction::Left, stroke),
        Icon::ArrowRight => arrow_shapes(inner, Direction::Right, stroke),
        Icon::AddRowAbove => add_shapes(square.shrink(side * 0.14), Direction::Up, stroke),
        Icon::AddRowBelow => add_shapes(square.shrink(side * 0.14), Direction::Down, stroke),
        Icon::AddColumnLeft => add_shapes(square.shrink(side * 0.14), Direction::Left, stroke),
        Icon::AddColumnRight => add_shapes(square.shrink(side * 0.14), Direction::Right, stroke),
    }
}

/// Paints `icon` into `rect`; returns the number of shapes drawn.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) -> usize {
    let shapes = icon_shapes(icon, rect, color);
    let count = shapes.len();
    painter.extend(shapes);
    count
}

/// The direction of a straight arrow icon.
#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn arrow_shapes(rect: Rect, direction: Direction, stroke: Stroke) -> Vec<Shape> {
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

fn add_shapes(rect: Rect, direction: Direction, stroke: Stroke) -> Vec<Shape> {
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

fn plus_shapes(center: Pos2, arm: f32, stroke: Stroke) -> Vec<Shape> {
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
fn arc_arrow(
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

#[cfg(test)]
mod tests {
    use egui::{Color32, Rect, pos2, vec2};

    use super::{Icon, icon_shapes};

    #[test]
    fn every_icon_paints_at_least_one_path_at_the_expected_size() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(44.0, 44.0));

        for icon in Icon::ALL {
            let shapes = icon_shapes(icon, rect, Color32::WHITE);
            assert!(!shapes.is_empty(), "{icon:?} painted nothing");
            let bounds = shapes.iter().fold(Rect::NOTHING, |bounds, shape| {
                bounds.union(shape.visual_bounding_rect())
            });
            assert!(
                rect.contains_rect(bounds),
                "{icon:?} escaped its cell: {bounds:?}"
            );
            assert!(
                bounds.width() > rect.width() * 0.3 && bounds.height() > 0.0,
                "{icon:?} is too small: {bounds:?}"
            );
        }
    }

    #[test]
    fn a_collapsed_cell_paints_nothing() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 44.0));

        assert!(icon_shapes(Icon::Back, rect, Color32::WHITE).is_empty());
    }
}

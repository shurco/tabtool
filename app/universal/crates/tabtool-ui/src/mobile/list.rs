//! The touch list row with swipe-to-delete and a rename context menu.

use egui::{
    Align2, Button, Color32, CornerRadius, FontId, Pos2, Rect, Response, RichText, Sense, Ui, Vec2,
    WidgetInfo, pos2,
};
use tabtool_i18n::{Language, tr};

use super::MobileMetrics;
use crate::fonts::heading_font;
use crate::theme::current;

/// The width of the revealed delete action.
const SWIPE_ACTION_WIDTH: f32 = 80.0;

/// How far a row must travel left before delete stays revealed.
const SWIPE_REVEAL_THRESHOLD: f32 = 40.0;

/// How far a pointer must move sideways before it counts as a swipe.
///
/// A smaller movement is a tap. The horizontal check keeps vertical drags free
/// for the enclosing scroll area.
const SWIPE_DRAG_THRESHOLD: f32 = 8.0;

/// The horizontal content inset of a list row.
const ROW_INSET: f32 = 16.0;

/// A two-line list row, e.g. for the Documents list.
pub struct ListRow<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    chevron: bool,
}

impl<'a> ListRow<'a> {
    /// A row titled `title`.
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            subtitle: None,
            chevron: false,
        }
    }

    /// Add the supporting subtitle line.
    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    /// Show the disclosure chevron for a row that opens a screen.
    pub fn chevron(mut self, chevron: bool) -> Self {
        self.chevron = chevron;
        self
    }

    /// Draw the row and report what the user asked for.
    pub fn show(self, ui: &mut Ui, metrics: MobileMetrics, language: Language) -> ListRowResponse {
        let theme = current(ui.ctx());
        let colors = theme.colors;
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, metrics.list_row_height), Sense::click());
        let revealed_id = response.id.with("list-row-revealed");
        let swipe_id = response.id.with("list-row-swipe");
        let mut revealed = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(revealed_id))
            .unwrap_or(false);
        let mut swipe = ui
            .ctx()
            .data(|data| data.get_temp::<SwipeState>(swipe_id))
            .unwrap_or_default();
        let pointer_pos = ui.input(|input| input.pointer.interact_pos());
        let pointer_down = ui.input(|input| input.pointer.any_down());
        let pointer_released = ui.input(|input| input.pointer.any_released());

        if response.is_pointer_button_down_on()
            && swipe.start.is_none()
            && let Some(position) = pointer_pos
        {
            swipe.start = Some(position);
        }
        if let Some(start) = swipe.start {
            if let Some(position) = pointer_pos {
                swipe.dx = position.x - start.x;
                swipe.dy = position.y - start.y;
                if swipe.dx.abs() > swipe.dy.abs() && swipe.dx.abs() >= SWIPE_DRAG_THRESHOLD {
                    swipe.horizontal = true;
                }
            }
            if pointer_down || pointer_released {
                ui.ctx().data_mut(|data| data.insert_temp(swipe_id, swipe));
            }
            if pointer_released {
                if swipe.horizontal {
                    if swipe.dx <= -SWIPE_REVEAL_THRESHOLD {
                        revealed = true;
                    } else if swipe.dx >= SWIPE_REVEAL_THRESHOLD {
                        revealed = false;
                    }
                }
                ui.ctx()
                    .data_mut(|data| data.insert_temp(revealed_id, revealed));
                swipe = SwipeState::default();
                ui.ctx().data_mut(|data| data.insert_temp(swipe_id, swipe));
            }
        }
        if swipe.start.is_some() && !pointer_down && !pointer_released {
            swipe = SwipeState::default();
            ui.ctx().data_mut(|data| data.insert_temp(swipe_id, swipe));
        }
        let swiping = swipe.start.is_some() && swipe.horizontal;
        let offset = if swiping {
            swipe.dx.clamp(-SWIPE_ACTION_WIDTH, 0.0)
        } else if revealed {
            -SWIPE_ACTION_WIDTH
        } else {
            0.0
        };

        let content = Rect::from_min_size(pos2(rect.left() + offset, rect.top()), rect.size());
        let background = if response.hovered() && !revealed {
            colors.hover_background
        } else {
            Color32::TRANSPARENT
        };
        let radius = CornerRadius::same(theme.metrics.small_corner_radius);
        ui.painter().rect_filled(rect, radius, background);

        let title_y = if self.subtitle.is_some() {
            rect.center().y - 8.0
        } else {
            rect.center().y
        };
        ui.painter().text(
            pos2(content.left() + ROW_INSET, title_y),
            Align2::LEFT_CENTER,
            self.title,
            FontId::proportional(metrics.body),
            colors.label,
        );
        if let Some(subtitle) = self.subtitle {
            ui.painter().text(
                pos2(content.left() + ROW_INSET, rect.center().y + 9.0),
                Align2::LEFT_CENTER,
                subtitle,
                FontId::proportional(metrics.caption),
                colors.secondary_label,
            );
            let subtitle_response =
                ui.interact(rect, response.id.with("list-row-subtitle"), Sense::hover());
            subtitle_response
                .widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, subtitle));
        }
        if self.chevron {
            ui.painter().text(
                pos2(content.right() - ROW_INSET, rect.center().y),
                Align2::RIGHT_CENTER,
                "\u{203A}",
                heading_font(20.0),
                colors.tertiary_label,
            );
        }
        ui.painter().hline(
            (rect.left() + ROW_INSET)..=rect.right(),
            rect.bottom(),
            egui::Stroke::new(1.0, colors.separator),
        );

        let mut activated = false;
        let mut delete_requested = false;
        let mut rename_requested = false;

        if revealed || swiping {
            let delete_rect = Rect::from_min_size(
                pos2(rect.right() - SWIPE_ACTION_WIDTH, rect.top()),
                Vec2::new(SWIPE_ACTION_WIDTH, rect.height()),
            );
            ui.painter().rect_filled(delete_rect, 0.0, colors.error);
            ui.painter().text(
                delete_rect.center(),
                Align2::CENTER_CENTER,
                tr(language, "Delete"),
                FontId::proportional(metrics.body),
                Color32::WHITE,
            );
            if revealed {
                let delete = ui.interact(
                    delete_rect,
                    response.id.with("list-row-delete"),
                    Sense::click(),
                );
                delete.widget_info(|| {
                    WidgetInfo::labeled(egui::Role::Button, true, tr(language, "Delete"))
                });
                if delete.clicked() {
                    delete_requested = true;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(revealed_id, false));
                }
            }
        }

        let long_touched = response.long_touched();
        if long_touched {
            rename_requested = true;
        }
        if response.clicked() && !long_touched && !delete_requested {
            let on_delete = response
                .interact_pointer_pos()
                .is_some_and(|pointer| pointer.x >= rect.right() - SWIPE_ACTION_WIDTH);
            if revealed && on_delete {
                delete_requested = true;
                ui.ctx()
                    .data_mut(|data| data.insert_temp(revealed_id, false));
            } else if revealed {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(revealed_id, false));
            } else {
                activated = true;
            }
        }

        response.context_menu(|ui| {
            let rename = Button::new(RichText::new(tr(language, "Rename")).size(metrics.body))
                .min_size(Vec2::new(180.0, metrics.control_height));
            if ui.add(rename).clicked() {
                rename_requested = true;
                ui.close();
            }
            let delete = Button::new(RichText::new(tr(language, "Delete")).size(metrics.body))
                .min_size(Vec2::new(180.0, metrics.control_height));
            if ui.add(delete).clicked() {
                delete_requested = true;
                ui.close();
            }
        });

        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, self.title));

        ListRowResponse {
            response,
            activated,
            delete_requested,
            rename_requested,
        }
    }
}

/// The pointer gesture in progress on one row.
#[derive(Clone, Copy, Debug, Default)]
struct SwipeState {
    start: Option<Pos2>,
    dx: f32,
    dy: f32,
    horizontal: bool,
}

/// What the user did to a list row.
pub struct ListRowResponse {
    /// The row's container response.
    pub response: Response,
    /// The row was tapped.
    pub activated: bool,
    /// Delete was chosen by swipe or the context menu.
    pub delete_requested: bool,
    /// Rename was chosen in the context menu.
    pub rename_requested: bool,
}

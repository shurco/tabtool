//! Inline banners for indexes, stale files and failed saves.

use egui::{Align, CornerRadius, Frame, Label, Layout, Margin, Response, RichText, Stroke, Ui};

use super::{ButtonKind, PushButton};
use crate::theme::current;

/// The severity of a banner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BannerKind {
    /// Informational.
    Info,
    /// Something needs attention but the file is still safe.
    Warning,
    /// An operation failed.
    Error,
}

/// The result of showing a banner.
pub struct BannerResponse {
    /// The banner's container response.
    pub response: Response,
    /// Whether the optional action button was pressed.
    pub action_clicked: bool,
}

/// Show a full-width banner with an icon, title, message and optional action.
pub fn banner(
    ui: &mut Ui,
    kind: BannerKind,
    title: &str,
    message: &str,
    action: Option<&str>,
) -> BannerResponse {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    let (background, foreground, icon) = match kind {
        BannerKind::Info => (colors.banner_info_background, colors.accent, "\u{2139}"),
        BannerKind::Warning => (colors.banner_warning_background, colors.warning, "\u{26A0}"),
        BannerKind::Error => (colors.banner_error_background, colors.error, "\u{2716}"),
    };
    let mut action_clicked = false;

    let frame = Frame::new()
        .fill(background)
        .stroke(Stroke::new(1.0, foreground.gamma_multiply(0.35)))
        .corner_radius(CornerRadius::same(theme.metrics.large_corner_radius))
        .inner_margin(Margin::same(theme.metrics.banner_padding as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(action) = action {
                    // The action takes the trailing edge first, so the wrapping
                    // text can never run underneath it on a narrow phone.
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if PushButton::new(action, ButtonKind::Bordered)
                            .show(ui)
                            .clicked()
                        {
                            action_clicked = true;
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            banner_text(ui, icon, title, message, foreground);
                        });
                    });
                } else {
                    banner_text(ui, icon, title, message, foreground);
                }
            });
        });

    BannerResponse {
        response: frame.response,
        action_clicked,
    }
}

fn banner_text(ui: &mut Ui, icon: &str, title: &str, message: &str, foreground: egui::Color32) {
    let theme = current(ui.ctx());
    let colors = theme.colors;
    ui.label(
        RichText::new(icon)
            .size(theme.typography.title)
            .color(foreground),
    );
    ui.vertical(|ui| {
        ui.add(
            Label::new(
                RichText::new(title)
                    .font(crate::fonts::heading_font(theme.typography.body))
                    .color(colors.label),
            )
            .wrap(),
        );
        if !message.is_empty() {
            ui.add(
                Label::new(
                    RichText::new(message)
                        .size(theme.typography.caption)
                        .color(colors.secondary_label),
                )
                .wrap(),
            );
        }
    });
}

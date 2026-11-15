//! The mapping of the desktop tokens onto egui's style.

use egui::{
    Context, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2, Visuals,
    style::WidgetVisuals,
};

use super::{Theme, ThemeMode, Typography, WINDOW_TITLE_STYLE};

impl Theme {
    /// Translate the tokens into egui's visual style.
    pub fn visuals(self) -> Visuals {
        let colors = self.colors;
        let mut visuals = match self.mode {
            ThemeMode::Light => Visuals::light(),
            ThemeMode::Dark => Visuals::dark(),
        };
        let border = Stroke::new(1.0, colors.control_border);
        let separator = Stroke::new(1.0, colors.separator);
        let radius = CornerRadius::same(self.metrics.corner_radius);

        visuals.dark_mode = self.is_dark();
        visuals.override_text_color = Some(colors.label);
        visuals.weak_text_color = Some(colors.secondary_label);
        visuals.panel_fill = colors.window_background;
        visuals.window_fill = colors.control_background;
        visuals.window_stroke = Stroke::new(1.0, colors.separator);
        visuals.extreme_bg_color = colors.content_background;
        visuals.text_edit_bg_color = Some(colors.control_background);
        visuals.faint_bg_color = colors.header_background;
        visuals.code_bg_color = colors.header_background;
        visuals.hyperlink_color = colors.accent;
        visuals.warn_fg_color = colors.warning;
        visuals.error_fg_color = colors.error;
        visuals.selection.bg_fill = colors.selection_background;
        visuals.selection.stroke = Stroke::new(1.0, colors.label);
        visuals.window_corner_radius = CornerRadius::same(self.metrics.large_corner_radius);
        visuals.menu_corner_radius = CornerRadius::same(self.metrics.large_corner_radius);

        visuals.widgets.noninteractive = WidgetVisuals {
            bg_fill: colors.window_background,
            weak_bg_fill: colors.window_background,
            bg_stroke: separator,
            corner_radius: radius,
            fg_stroke: Stroke::new(1.0, colors.label),
            expansion: 0.0,
        };
        visuals.widgets.inactive = WidgetVisuals {
            bg_fill: colors.control_background,
            weak_bg_fill: colors.control_background,
            bg_stroke: border,
            corner_radius: radius,
            fg_stroke: Stroke::new(1.0, colors.label),
            expansion: 0.0,
        };
        visuals.widgets.hovered = WidgetVisuals {
            bg_fill: colors.hover_background,
            weak_bg_fill: colors.hover_background,
            bg_stroke: border,
            corner_radius: radius,
            fg_stroke: Stroke::new(1.0, colors.label),
            expansion: 0.0,
        };
        visuals.widgets.active = WidgetVisuals {
            bg_fill: colors.hover_background,
            weak_bg_fill: colors.hover_background,
            bg_stroke: border,
            corner_radius: radius,
            fg_stroke: Stroke::new(1.0, colors.label),
            expansion: 0.0,
        };
        visuals.widgets.open = WidgetVisuals {
            bg_fill: colors.hover_background,
            weak_bg_fill: colors.hover_background,
            bg_stroke: Stroke::new(1.0, colors.accent),
            corner_radius: radius,
            fg_stroke: Stroke::new(1.0, colors.label),
            expansion: 0.0,
        };
        visuals
    }

    pub(super) fn apply(self, ctx: &Context) {
        let mode = self.mode.egui();
        let mut style = ctx.style_of(mode).as_ref().clone();
        style.visuals = self.visuals();
        style.spacing.item_spacing = Vec2::splat(self.metrics.item_spacing);
        style.spacing.interact_size = Vec2::new(0.0, self.metrics.control_height);
        style.spacing.button_padding =
            Vec2::new(self.metrics.button_padding_x, self.metrics.button_padding_y);
        style.spacing.window_margin = Margin::same(self.metrics.window_padding as i8);
        style.spacing.menu_margin = Margin::same(4);
        style.spacing.indent = 16.0;
        style.spacing.combo_width = 120.0;
        style.spacing.text_edit_width = 160.0;
        style.text_styles = text_styles(self.typography);
        ctx.set_style_of(mode, style);
    }
}

fn text_styles(typography: Typography) -> std::collections::BTreeMap<TextStyle, FontId> {
    let proportional = |size| FontId::new(size, FontFamily::Proportional);
    let heading = |size| FontId::new(size, FontFamily::Name(crate::fonts::HEADING_FAMILY.into()));
    [
        (TextStyle::Body, proportional(typography.body)),
        (TextStyle::Button, proportional(typography.body)),
        (TextStyle::Small, proportional(typography.caption)),
        (TextStyle::Heading, heading(typography.title)),
        (
            TextStyle::Name(WINDOW_TITLE_STYLE.into()),
            heading(typography.title_large),
        ),
        (
            TextStyle::Monospace,
            FontId::new(typography.body, FontFamily::Monospace),
        ),
    ]
    .into()
}

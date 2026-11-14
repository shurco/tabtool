//! The TabTool desktop design tokens and their mapping onto egui's style.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2, Visuals,
    style::WidgetVisuals,
};

/// The light or dark appearance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ThemeMode {
    /// The light macOS appearance.
    Light,
    /// The dark macOS appearance.
    Dark,
}

impl ThemeMode {
    /// The matching egui theme.
    pub fn egui(self) -> egui::Theme {
        match self {
            Self::Light => egui::Theme::Light,
            Self::Dark => egui::Theme::Dark,
        }
    }
}

impl From<egui::Theme> for ThemeMode {
    fn from(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Light => Self::Light,
            egui::Theme::Dark => Self::Dark,
        }
    }
}

/// Every color the desktop UI uses.
///
/// The values reproduce the compact macOS appearance; the accent is the
/// system blue of each appearance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Colors {
    /// The system accent used for prominent buttons, focus rings and selections.
    pub accent: Color32,
    /// The accent while a prominent control is pressed.
    pub accent_pressed: Color32,
    /// Primary text.
    pub label: Color32,
    /// Supporting text, e.g. captions and status lines.
    pub secondary_label: Color32,
    /// The quietest text, e.g. placeholders.
    pub tertiary_label: Color32,
    /// Hairlines between rows, bars and columns.
    pub separator: Color32,
    /// The fill of a text field or an inactive push button.
    pub control_background: Color32,
    /// The raised surface of a selected segment, lighter than its track.
    pub raised_background: Color32,
    /// The border of a text field or a bordered push button.
    pub control_border: Color32,
    /// The window and panel background.
    pub window_background: Color32,
    /// The document surface background.
    pub content_background: Color32,
    /// The grid header and tab strip background.
    pub header_background: Color32,
    /// The fill behind selected rows and text.
    pub selection_background: Color32,
    /// The fill of a hovered row or plain button.
    pub hover_background: Color32,
    /// The tint of an alternating data row.
    pub alternating_row_background: Color32,
    /// The ring around a keyboard-focused control.
    pub focus_ring: Color32,
    /// Destructive text and errors.
    pub error: Color32,
    /// Warning text.
    pub warning: Color32,
    /// Success text.
    pub success: Color32,
    /// The background of an informational banner.
    pub banner_info_background: Color32,
    /// The background of a warning banner.
    pub banner_warning_background: Color32,
    /// The background of an error banner.
    pub banner_error_background: Color32,
}

/// The compact macOS type scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    /// Body text, 13 pt.
    pub body: f32,
    /// Captions and status text, 11 pt.
    pub caption: f32,
    /// Section titles, 15 pt.
    pub title: f32,
    /// Window titles, 17 pt.
    pub title_large: f32,
}

/// The named text style for large window titles.
pub const WINDOW_TITLE_STYLE: &str = "WindowTitle";

/// Desktop control geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The height of a standard control, 22 pt.
    pub control_height: f32,
    /// The height of a large control, 24 pt.
    pub control_height_large: f32,
    /// The default grid row height, 24 pt.
    pub row_height: f32,
    /// The radius of small controls, 4 pt.
    pub small_corner_radius: u8,
    /// The radius of standard controls, 5 pt.
    pub corner_radius: u8,
    /// The radius of windows, sheets and banners, 6 pt.
    pub large_corner_radius: u8,
    /// The gap between related controls, 8 pt.
    pub item_spacing: f32,
    /// The window and sheet content inset, 10 pt.
    pub window_padding: f32,
    /// Horizontal push-button padding, 10 pt.
    pub button_padding_x: f32,
    /// Vertical push-button padding, 3 pt.
    pub button_padding_y: f32,
    /// The height of a menu row, 22 pt.
    pub menu_item_height: f32,
    /// The width of the keyboard focus ring, 2 pt.
    pub focus_ring_width: f32,
    /// The height of the toolbar strip, 32 pt.
    pub toolbar_height: f32,
    /// Horizontal toolbar inset, 10 pt.
    pub toolbar_padding_x: f32,
    /// Vertical toolbar inset, 5 pt.
    pub toolbar_padding_y: f32,
    /// The height of the status line, 22 pt.
    pub status_line_height: f32,
    /// Horizontal status-line inset, 10 pt.
    pub status_padding_x: f32,
    /// Vertical status-line inset, 3 pt.
    pub status_padding_y: f32,
    /// Modal alert content inset, 16 pt.
    pub alert_padding: f32,
    /// Modal alert width, 340 pt.
    pub alert_width: f32,
    /// Sheet content inset, 16 pt.
    pub sheet_padding: f32,
    /// Sheet width, 440 pt.
    pub sheet_width: f32,
    /// Banner content inset, 10 pt.
    pub banner_padding: f32,
}

/// One complete desktop appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// Light or dark.
    pub mode: ThemeMode,
    /// The palette.
    pub colors: Colors,
    /// The type scale.
    pub typography: Typography,
    /// The control geometry.
    pub metrics: Metrics,
}

const LIGHT: Colors = Colors {
    accent: rgb(0x007AFF),
    accent_pressed: rgb(0x0062CC),
    label: rgb(0x1D1D1F),
    secondary_label: rgb(0x6E6E73),
    tertiary_label: rgb(0x8E8E93),
    separator: rgb(0xE5E5E7),
    control_background: rgb(0xFFFFFF),
    raised_background: rgb(0xFFFFFF),
    control_border: rgb(0xD1D1D6),
    window_background: rgb(0xFFFFFF),
    content_background: rgb(0xFFFFFF),
    header_background: rgb(0xEBEBEB),
    selection_background: rgb(0xB3D7FF),
    hover_background: rgb(0xF2F2F2),
    alternating_row_background: rgb(0xF7F7F9),
    focus_ring: rgb(0x007AFF),
    error: rgb(0xD70015),
    warning: rgb(0xB25000),
    success: rgb(0x248A3D),
    banner_info_background: rgb(0xEAF3FF),
    banner_warning_background: rgb(0xFFF4E5),
    banner_error_background: rgb(0xFDECEA),
};

const DARK: Colors = Colors {
    accent: rgb(0x0A84FF),
    accent_pressed: rgb(0x409CFF),
    label: rgb(0xFFFFFF),
    secondary_label: rgb(0xA1A1A6),
    tertiary_label: rgb(0x8E8E93),
    separator: rgb(0x3A3A3C),
    control_background: rgb(0x1C1C1E),
    raised_background: rgb(0x48484A),
    control_border: rgb(0x48484A),
    window_background: rgb(0x1E1E1E),
    content_background: rgb(0x1E1E1E),
    header_background: rgb(0x2C2C2E),
    selection_background: rgb(0x0A84FF),
    hover_background: rgb(0x2C2C2E),
    alternating_row_background: rgb(0x252527),
    focus_ring: rgb(0x0A84FF),
    error: rgb(0xFF453A),
    warning: rgb(0xFF9F0A),
    success: rgb(0x32D74B),
    banner_info_background: rgb(0x152538),
    banner_warning_background: rgb(0x33260F),
    banner_error_background: rgb(0x3A1A18),
};

const TYPOGRAPHY: Typography = Typography {
    body: 13.0,
    caption: 11.0,
    title: 15.0,
    title_large: 17.0,
};

const METRICS: Metrics = Metrics {
    control_height: 22.0,
    control_height_large: 24.0,
    row_height: 24.0,
    small_corner_radius: 4,
    corner_radius: 5,
    large_corner_radius: 6,
    item_spacing: 8.0,
    window_padding: 10.0,
    button_padding_x: 10.0,
    button_padding_y: 3.0,
    menu_item_height: 22.0,
    focus_ring_width: 2.0,
    toolbar_height: 32.0,
    toolbar_padding_x: 10.0,
    toolbar_padding_y: 5.0,
    status_line_height: 22.0,
    status_padding_x: 10.0,
    status_padding_y: 3.0,
    alert_padding: 16.0,
    alert_width: 340.0,
    sheet_padding: 16.0,
    sheet_width: 440.0,
    banner_padding: 10.0,
};

const fn rgb(value: u32) -> Color32 {
    Color32::from_rgb(
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    )
}

impl Theme {
    /// The desktop tokens for an appearance.
    pub fn desktop(mode: ThemeMode) -> Self {
        Self {
            mode,
            colors: match mode {
                ThemeMode::Light => LIGHT,
                ThemeMode::Dark => DARK,
            },
            typography: TYPOGRAPHY,
            metrics: METRICS,
        }
    }

    /// The compact light macOS appearance.
    pub fn desktop_light() -> Self {
        Self::desktop(ThemeMode::Light)
    }

    /// The compact dark macOS appearance.
    pub fn desktop_dark() -> Self {
        Self::desktop(ThemeMode::Dark)
    }

    /// Is this the dark appearance?
    pub fn is_dark(self) -> bool {
        self.mode == ThemeMode::Dark
    }

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

    fn apply(self, ctx: &Context) {
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

fn theme_id(mode: ThemeMode) -> egui::Id {
    egui::Id::unique(("tabtool-ui-theme", mode))
}

/// The tokens currently installed for the active appearance.
///
/// Widgets use this to reach the label, separator and banner colors that egui's
/// `Visuals` does not model. A context without an installed theme reads the
/// built-in tokens for its current appearance.
pub fn current(ctx: &Context) -> Theme {
    let mode = ThemeMode::from(ctx.theme());
    ctx.data(|data| data.get_temp(theme_id(mode)))
        .unwrap_or_else(|| Theme::desktop(mode))
}

fn store(ctx: &Context, theme: Theme) {
    ctx.data_mut(|data| data.insert_temp(theme_id(theme.mode), theme));
}

/// Install one token set and make its appearance active.
///
/// The tokens apply to the matching egui appearance (light or dark); the other
/// appearance keeps its defaults. Use [`apply_themes`] to install both and
/// follow the system appearance.
pub fn apply_theme(ctx: &Context, theme: &Theme) {
    ctx.set_theme(theme.mode.egui());
    theme.apply(ctx);
    store(ctx, *theme);
}

/// Install both built-in appearances and follow the system light/dark setting.
pub fn apply_themes(ctx: &Context) {
    for theme in [Theme::desktop_light(), Theme::desktop_dark()] {
        theme.apply(ctx);
        store(ctx, theme);
    }
    ctx.set_theme(egui::ThemePreference::System);
}

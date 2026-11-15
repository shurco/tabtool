//! The TabTool desktop design tokens and their mapping onto egui's style.

mod style;
mod tokens;

use egui::{Color32, Context};

use tokens::{DARK, LIGHT, METRICS, TYPOGRAPHY};

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

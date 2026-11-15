//! The built-in desktop token values.

use egui::Color32;

use super::{Colors, Metrics, Typography};

pub(super) const LIGHT: Colors = Colors {
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

pub(super) const DARK: Colors = Colors {
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

pub(super) const TYPOGRAPHY: Typography = Typography {
    body: 13.0,
    caption: 11.0,
    title: 15.0,
    title_large: 17.0,
};

pub(super) const METRICS: Metrics = Metrics {
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

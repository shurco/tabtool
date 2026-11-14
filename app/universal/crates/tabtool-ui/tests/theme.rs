//! Design tokens and system fonts.

use egui::{Color32, Context, FontFamily, FontId, TextStyle};
use tabtool_ui::{
    Theme, apply_theme, apply_themes, install_fonts,
    theme::{self, ThemeMode},
};

fn rgb(value: u32) -> Color32 {
    Color32::from_rgb(
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    )
}

/// Fonts are created at the start of a pass, so the tests run one empty pass.
fn run_pass(ctx: &Context) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |_| {});
    output.textures_delta.clear();
}

#[test]
fn light_palette_pins_the_system_colors() {
    let theme = Theme::desktop_light();

    assert_eq!(theme.mode, ThemeMode::Light);
    assert_eq!(theme.colors.accent, rgb(0x007AFF));
    assert_eq!(theme.colors.accent_pressed, rgb(0x0062CC));
    assert_eq!(theme.colors.label, rgb(0x1D1D1F));
    assert_eq!(theme.colors.secondary_label, rgb(0x6E6E73));
    assert_eq!(theme.colors.tertiary_label, rgb(0x8E8E93));
    assert_eq!(theme.colors.separator, rgb(0xE5E5E7));
    assert_eq!(theme.colors.control_background, rgb(0xFFFFFF));
    assert_eq!(theme.colors.raised_background, rgb(0xFFFFFF));
    assert_eq!(theme.colors.control_border, rgb(0xD1D1D6));
    assert_eq!(theme.colors.window_background, rgb(0xFFFFFF));
    assert_eq!(theme.colors.content_background, rgb(0xFFFFFF));
    assert_eq!(theme.colors.header_background, rgb(0xEBEBEB));
    assert_eq!(theme.colors.selection_background, rgb(0xB3D7FF));
    assert_eq!(theme.colors.hover_background, rgb(0xF2F2F2));
    assert_eq!(theme.colors.alternating_row_background, rgb(0xF7F7F9));
    assert_eq!(theme.colors.focus_ring, rgb(0x007AFF));
    assert_eq!(theme.colors.error, rgb(0xD70015));
    assert_eq!(theme.colors.warning, rgb(0xB25000));
    assert_eq!(theme.colors.success, rgb(0x248A3D));
}

#[test]
fn dark_palette_pins_the_system_colors() {
    let theme = Theme::desktop_dark();

    assert_eq!(theme.mode, ThemeMode::Dark);
    assert_eq!(theme.colors.accent, rgb(0x0A84FF));
    assert_eq!(theme.colors.accent_pressed, rgb(0x409CFF));
    assert_eq!(theme.colors.label, rgb(0xFFFFFF));
    assert_eq!(theme.colors.secondary_label, rgb(0xA1A1A6));
    assert_eq!(theme.colors.tertiary_label, rgb(0x8E8E93));
    assert_eq!(theme.colors.separator, rgb(0x3A3A3C));
    assert_eq!(theme.colors.control_background, rgb(0x1C1C1E));
    assert_eq!(theme.colors.raised_background, rgb(0x48484A));
    assert_eq!(theme.colors.control_border, rgb(0x48484A));
    assert_eq!(theme.colors.window_background, rgb(0x1E1E1E));
    assert_eq!(theme.colors.content_background, rgb(0x1E1E1E));
    assert_eq!(theme.colors.header_background, rgb(0x2C2C2E));
    assert_eq!(theme.colors.selection_background, rgb(0x0A84FF));
    assert_eq!(theme.colors.hover_background, rgb(0x2C2C2E));
    assert_eq!(theme.colors.alternating_row_background, rgb(0x252527));
    assert_eq!(theme.colors.focus_ring, rgb(0x0A84FF));
    assert_eq!(theme.colors.error, rgb(0xFF453A));
    assert_eq!(theme.colors.warning, rgb(0xFF9F0A));
    assert_eq!(theme.colors.success, rgb(0x32D74B));
}

#[test]
fn typography_pins_the_compact_macos_sizes() {
    let typography = Theme::desktop_light().typography;

    assert_eq!(typography.body, 13.0);
    assert_eq!(typography.caption, 11.0);
    assert_eq!(typography.title, 15.0);
    assert_eq!(typography.title_large, 17.0);
}

#[test]
fn metrics_pin_the_desktop_control_sizes() {
    let metrics = Theme::desktop_light().metrics;

    assert_eq!(metrics.control_height, 22.0);
    assert_eq!(metrics.control_height_large, 24.0);
    assert_eq!(metrics.row_height, 24.0);
    assert_eq!(metrics.small_corner_radius, 4);
    assert_eq!(metrics.corner_radius, 5);
    assert_eq!(metrics.large_corner_radius, 6);
    assert_eq!(metrics.item_spacing, 8.0);
    assert_eq!(metrics.window_padding, 10.0);
    assert_eq!(metrics.button_padding_x, 10.0);
    assert_eq!(metrics.button_padding_y, 3.0);
    assert_eq!(metrics.menu_item_height, 22.0);
    assert_eq!(metrics.focus_ring_width, 2.0);
    assert_eq!(metrics.toolbar_height, 32.0);
    assert_eq!(metrics.toolbar_padding_x, 10.0);
    assert_eq!(metrics.toolbar_padding_y, 5.0);
    assert_eq!(metrics.status_line_height, 22.0);
    assert_eq!(metrics.status_padding_x, 10.0);
    assert_eq!(metrics.status_padding_y, 3.0);
    assert_eq!(metrics.alert_padding, 16.0);
    assert_eq!(metrics.alert_width, 340.0);
    assert_eq!(metrics.sheet_padding, 16.0);
    assert_eq!(metrics.sheet_width, 440.0);
    assert_eq!(metrics.banner_padding, 10.0);
}

#[test]
fn dark_mode_only_swaps_colors() {
    let light = Theme::desktop_light();
    let dark = Theme::desktop_dark();

    assert_eq!(light.typography, dark.typography);
    assert_eq!(light.metrics, dark.metrics);
    assert_ne!(light.colors, dark.colors);
}

#[test]
fn apply_theme_writes_the_tokens_into_the_egui_style() {
    let ctx = Context::default();
    let theme = Theme::desktop_dark();

    apply_theme(&ctx, &theme);

    assert_eq!(ctx.theme(), egui::Theme::Dark);
    let style = ctx.style_of(egui::Theme::Dark);
    assert_eq!(style.visuals.panel_fill, theme.colors.window_background);
    assert_eq!(style.visuals.window_fill, theme.colors.control_background);
    assert_eq!(
        style.visuals.selection.bg_fill,
        theme.colors.selection_background
    );
    assert_eq!(style.text_styles[&TextStyle::Body].size, 13.0);
    assert_eq!(style.text_styles[&TextStyle::Heading].size, 15.0);
    assert_eq!(
        style.text_styles[&TextStyle::Name(tabtool_ui::theme::WINDOW_TITLE_STYLE.into())].size,
        17.0
    );
    assert_eq!(style.spacing.interact_size.y, 22.0);
    assert_eq!(style.visuals.strong_text_color(), theme.colors.label);
    assert_eq!(style.visuals.text_color(), theme.colors.label);
    assert_eq!(theme::current(&ctx), theme);
}

#[test]
fn apply_themes_installs_both_palettes() {
    let ctx = Context::default();

    apply_themes(&ctx);

    let light = Theme::desktop_light();
    let dark = Theme::desktop_dark();
    assert_eq!(
        ctx.style_of(egui::Theme::Light).visuals.panel_fill,
        light.colors.window_background
    );
    assert_eq!(
        ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
        dark.colors.window_background
    );

    ctx.set_theme(egui::Theme::Light);
    assert_eq!(theme::current(&ctx), light);
    ctx.set_theme(egui::Theme::Dark);
    assert_eq!(theme::current(&ctx), dark);
}

#[test]
fn system_fonts_shape_latin_text() {
    let ctx = Context::default();
    install_fonts(&ctx).expect("the system must provide fonts");
    run_pass(&ctx);

    let body = FontId::proportional(13.0);
    ctx.fonts_mut(|fonts| {
        assert!(
            fonts.has_glyph(&body, 'A'),
            "the system sans-serif family must cover Latin text"
        );
    });
}

#[test]
fn font_families_resolve_in_a_deterministic_order() {
    let ctx = Context::default();
    install_fonts(&ctx).expect("the system must provide fonts");
    run_pass(&ctx);

    let families = ctx.fonts(|fonts| fonts.definitions().families.clone());
    let proportional = &families[&FontFamily::Proportional];
    let monospace = &families[&FontFamily::Monospace];
    let heading = &families[&FontFamily::Name(tabtool_ui::fonts::HEADING_FAMILY.into())];

    assert!(!proportional.is_empty(), "proportional must resolve");
    assert!(!monospace.is_empty(), "monospace must resolve");
    assert!(!heading.is_empty(), "the heading family must resolve");

    assert!(
        proportional[0].contains(":SansSerif:"),
        "proportional must start with system sans-serif, got {proportional:?}"
    );
    assert!(
        proportional.iter().any(|name| name.contains(":Serif:")),
        "serif must follow sans-serif, got {proportional:?}"
    );
    assert!(
        proportional.iter().any(|name| name.contains(":Monospace:")),
        "monospace must be the last fallback, got {proportional:?}"
    );
    assert!(
        monospace[0].contains(":Monospace:"),
        "monospace must start with the system monospace face, got {monospace:?}"
    );
    assert!(
        heading == proportional || heading.ends_with(proportional),
        "the heading family must be the proportional list or extend it with a \
         heavier face first, got {heading:?}"
    );
    if heading != proportional {
        assert!(
            heading.len() > proportional.len(),
            "a heavier heading face must be added before the proportional \
             fallbacks, got {heading:?}"
        );
    }
    assert!(
        ctx.fonts(|fonts| fonts
            .definitions()
            .font_data
            .keys()
            .all(|name| name.starts_with("system:") || name.starts_with("system-heading:"))),
        "no bundled font may be installed"
    );
}

#[test]
fn heading_text_style_uses_the_heading_family() {
    let ctx = Context::default();
    install_fonts(&ctx).expect("the system must provide fonts");
    apply_theme(&ctx, &Theme::desktop_light());

    let heading = ctx.style_of(egui::Theme::Light).text_styles[&TextStyle::Heading].clone();

    assert_eq!(
        heading.family,
        FontFamily::Name(tabtool_ui::fonts::HEADING_FAMILY.into())
    );
}

#[test]
fn the_startup_coverage_check_reports_exactly_the_uncovered_scripts() {
    let ctx = Context::default();
    install_fonts(&ctx).expect("the system must provide fonts");
    run_pass(&ctx);

    let missing = tabtool_ui::fonts::warn_missing_scripts(&ctx);

    for script in tabtool_ui::fonts::Script::ALL {
        let covered = ctx.fonts_mut(|fonts| {
            script.sample().chars().all(|character| {
                [FontId::proportional(13.0), FontId::monospace(13.0)]
                    .iter()
                    .any(|font| fonts.has_glyph(font, character))
            })
        });

        assert_eq!(
            missing.contains(&script),
            !covered,
            "{} must be reported only when neither family covers it",
            script.name()
        );
    }
}

#[test]
fn no_font_files_are_bundled_with_the_ui() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");

    assert!(
        !assets.exists(),
        "bundled font assets must not return; system fonts are used instead: {}",
        assets.display()
    );
}

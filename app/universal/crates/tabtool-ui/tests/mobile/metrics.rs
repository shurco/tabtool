//! Touch metrics, row-height mapping and safe-area application.

use egui::{Rect, pos2};
use tabtool_i18n::{Language, tr_in};
use tabtool_settings::valid_mobile_row_height;
use tabtool_ui::mobile::{
    Insets, MOBILE_BODY_TEXT, MOBILE_DEFAULT_ROW_HEIGHT, MOBILE_MIN_TOUCH_TARGET,
    MOBILE_ROW_HEIGHTS, MobileMetrics,
};

fn available() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(360.0, 800.0))
}

#[test]
fn the_phone_uses_the_ios_type_and_touch_scale() {
    let metrics = MobileMetrics::phone();

    assert_eq!(metrics.body, 17.0);
    assert_eq!(metrics.caption, 11.0);
    assert_eq!(metrics.title, 17.0);
    assert_eq!(metrics.row_height, 44.0);
    assert_eq!(metrics.control_height, 44.0);
    assert_eq!(metrics.min_touch_target, 44.0);
    assert_eq!(metrics.safe_area, Insets::ZERO);
    assert_eq!(metrics.sheet_max_width, None);
}

#[test]
fn the_tablet_keeps_the_mobile_layout_and_widens_sheets() {
    let phone = MobileMetrics::phone();
    let tablet = MobileMetrics::tablet();

    assert_eq!(tablet.body, phone.body);
    assert_eq!(tablet.row_height, phone.row_height);
    assert_eq!(tablet.control_height, phone.control_height);
    assert_eq!(tablet.sheet_max_width, Some(540.0));
}

#[test]
fn every_touch_target_is_at_least_forty_four_points() {
    for metrics in [MobileMetrics::phone(), MobileMetrics::tablet()] {
        for value in [
            metrics.control_height,
            metrics.toolbar_height,
            metrics.action_bar_height,
            metrics.accessory_height,
            metrics.list_row_height,
            metrics.min_touch_target,
        ] {
            assert!(
                value >= MOBILE_MIN_TOUCH_TARGET,
                "touch target {value} is smaller than the minimum"
            );
        }
        assert_eq!(metrics.body, MOBILE_BODY_TEXT);
    }
}

#[test]
fn the_row_heights_are_the_three_mobile_options() {
    assert_eq!(MOBILE_ROW_HEIGHTS, [32.0, 44.0, 56.0]);
    assert_eq!(MOBILE_DEFAULT_ROW_HEIGHT, 44.0);

    assert_eq!(valid_mobile_row_height(32.0), 32.0);
    assert_eq!(valid_mobile_row_height(44.0), 44.0);
    assert_eq!(valid_mobile_row_height(56.0), 56.0);
}

#[test]
fn an_unknown_stored_row_height_falls_back_to_the_default() {
    for stored in [20.0, 24.0, 0.0, -1.0, 40.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            valid_mobile_row_height(stored),
            MOBILE_DEFAULT_ROW_HEIGHT,
            "stored height {stored} must fall back"
        );
    }
    assert_eq!(
        MobileMetrics::phone().with_row_height(56.0).row_height,
        56.0
    );
    assert_eq!(
        MobileMetrics::phone().with_row_height(20.0).row_height,
        MOBILE_DEFAULT_ROW_HEIGHT
    );
}

#[test]
fn safe_area_insets_move_the_content_inside_the_bars() {
    let metrics = MobileMetrics::phone().with_safe_area(Insets::from_array([0.0, 24.0, 0.0, 24.0]));
    let content = metrics.content_rect(available());

    assert_eq!(content.min, pos2(0.0, 24.0));
    assert_eq!(content.max, pos2(360.0, 776.0));
}

#[test]
fn landscape_and_cutout_insets_apply_on_every_edge() {
    let metrics = MobileMetrics::phone().with_safe_area(Insets::new(48.0, 0.0, 44.0, 20.0));
    let content = metrics.content_rect(available());

    assert_eq!(content.min, pos2(48.0, 0.0));
    assert_eq!(content.max, pos2(316.0, 780.0));
}

#[test]
fn insets_larger_than_the_window_never_invert_the_content() {
    let metrics = MobileMetrics::phone().with_safe_area(Insets::new(400.0, 500.0, 400.0, 500.0));
    let content = metrics.content_rect(available());

    assert!(content.width() >= 0.0);
    assert!(content.height() >= 0.0);
    assert_eq!(content.min, pos2(400.0, 500.0));
    assert_eq!(content.max, content.min);
}

#[test]
fn the_keyboard_wins_over_the_bottom_safe_area() {
    let metrics = MobileMetrics::phone().with_safe_area(Insets::from_array([0.0, 24.0, 0.0, 24.0]));

    assert_eq!(metrics.bottom_inset(0.0), 24.0);
    assert_eq!(metrics.bottom_inset(300.0), 300.0);
    assert_eq!(metrics.bottom_inset(10.0), 24.0);
}

#[test]
fn a_sheet_fills_a_phone_and_caps_itself_on_a_tablet() {
    let phone = MobileMetrics::phone();
    let tablet = MobileMetrics::tablet();

    assert_eq!(phone.sheet_width(360.0), 360.0);
    assert_eq!(phone.sheet_width(800.0), 800.0);
    assert_eq!(tablet.sheet_width(800.0), 540.0);
    assert_eq!(tablet.sheet_width(390.0), 390.0);
}

#[test]
fn the_sheet_height_leaves_room_for_the_safe_area_and_keyboard() {
    let metrics = MobileMetrics::phone().with_safe_area(Insets::from_array([0.0, 24.0, 0.0, 24.0]));

    assert_eq!(metrics.sheet_max_height(800.0, 0.0), 752.0);
    assert_eq!(metrics.sheet_max_height(800.0, 300.0), 476.0);
}

#[test]
fn every_mobile_label_is_in_the_catalog_for_every_language() {
    use tabtool_ui::mobile::MISSING_DOCUMENT_NOTICE;

    let keys = [
        MISSING_DOCUMENT_NOTICE,
        "Documents",
        "Undo",
        "Redo",
        "Filter",
        "Export",
        "More",
        "Format",
        "Settings",
        "About",
        "Save As",
        "Add Row Above",
        "Add Row Below",
        "Add Column Left",
        "Add Column Right",
        "Delete Selection",
        "Sort Ascending",
        "Sort Descending",
        "Remove Sort",
        "Rename Column",
        "Previous Cell",
        "Next Cell",
        "Done",
        "Rename",
        "Delete",
    ];

    for key in keys {
        for language in Language::ALL {
            if language == Language::System {
                continue;
            }
            assert!(
                tr_in(language, key).is_some(),
                "missing {language:?} translation for `{key}`"
            );
        }
    }
}

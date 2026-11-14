//! Contextual action derivation, live-selection enablement and fold splitting.

use core::cell::RefCell;
use std::rc::Rc;

use egui::{Rect, pos2, vec2};
use tabtool_core::{CsvConfiguration, Locale, SortDirection};
use tabtool_document::Document;
use tabtool_i18n::{Language, tr};
use tabtool_ui::grid::GridSelection;
use tabtool_ui::harness::kittest::Queryable;
use tabtool_ui::mobile::{
    ContextualAction, ContextualActionId, FoldGap, FoldRegion, MobileMetrics, actions_for,
    actions_for_selection, contextual_action_bar, fold_gap, is_enabled,
};
use tabtool_ui::{Theme, UiHarness};

const EN: Language = Language::En;

fn document() -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(
            b"Name,Amount\nAlice,1.5\nBob,20\nCarol,-3.25\n",
            None,
            Some(configuration),
            false,
        )
        .expect("the fixture parses");
    document
}

fn selection(rows: &[usize], columns: &[usize]) -> GridSelection {
    GridSelection {
        rows: rows.iter().copied().collect(),
        columns: columns.iter().copied().collect(),
    }
}

fn bar(width: f32) -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 44.0))
}

fn ids(actions: &[ContextualAction]) -> Vec<ContextualActionId> {
    actions.iter().map(|action| action.id).collect()
}

fn enabled(actions: &[ContextualAction], id: ContextualActionId) -> bool {
    actions
        .iter()
        .find(|action| action.id == id)
        .is_some_and(|action| action.enabled)
}

fn column_actions() -> Vec<ContextualAction> {
    actions_for_selection(&selection(&[], &[0]), true)
        .into_iter()
        .map(|id| ContextualAction { id, enabled: true })
        .collect()
}

const ROW_ACTIONS: [ContextualActionId; 3] = [
    ContextualActionId::AddRowAbove,
    ContextualActionId::AddRowBelow,
    ContextualActionId::Delete,
];

const COLUMN_ACTIONS: [ContextualActionId; 7] = [
    ContextualActionId::AddColumnLeft,
    ContextualActionId::AddColumnRight,
    ContextualActionId::Delete,
    ContextualActionId::SortAscending,
    ContextualActionId::SortDescending,
    ContextualActionId::RemoveSort,
    ContextualActionId::RenameColumn,
];

#[test]
fn rows_offer_the_three_row_actions() {
    assert_eq!(
        actions_for_selection(&selection(&[1], &[]), true),
        ROW_ACTIONS
    );
}

#[test]
fn columns_offer_the_seven_column_actions() {
    assert_eq!(
        actions_for_selection(&selection(&[], &[0]), true),
        COLUMN_ACTIONS
    );
}

#[test]
fn nothing_selected_or_no_structural_edits_offers_nothing() {
    assert!(actions_for_selection(&GridSelection::default(), true).is_empty());
    assert!(actions_for_selection(&selection(&[1], &[]), false).is_empty());
}

#[test]
fn the_live_grid_selection_drives_the_actions() {
    let document = document();
    assert!(
        document.selection().rows.is_empty() && document.selection().columns.is_empty(),
        "the document mirror starts empty"
    );

    let rows = actions_for(&document, &selection(&[1], &[]));
    assert_eq!(ids(&rows), ROW_ACTIONS);
    assert!(rows.iter().all(|action| action.enabled));

    let columns = actions_for(&document, &selection(&[], &[0]));
    assert_eq!(ids(&columns), COLUMN_ACTIONS);
    assert!(enabled(&columns, ContextualActionId::AddColumnLeft));
}

#[test]
fn a_stale_document_mirror_does_not_hide_the_live_selection() {
    let mut document = document();
    document.selection_mut().rows.insert(0);
    assert!(
        !document.selection().rows.is_empty(),
        "the mirror holds the old row selection"
    );

    let columns = actions_for(&document, &selection(&[], &[1]));
    assert_eq!(ids(&columns), COLUMN_ACTIONS);
    assert!(enabled(&columns, ContextualActionId::AddColumnLeft));
    assert!(enabled(&columns, ContextualActionId::SortAscending));
}

#[test]
fn a_row_selection_enables_the_row_actions() {
    let document = document();

    let actions = actions_for(&document, &selection(&[0], &[]));

    assert_eq!(ids(&actions), ROW_ACTIONS);
    assert!(actions.iter().all(|action| action.enabled));
}

#[test]
fn a_column_selection_enables_the_column_actions_and_remove_sort_waits() {
    let mut document = document();

    let actions = actions_for(&document, &selection(&[], &[0]));

    assert_eq!(ids(&actions), COLUMN_ACTIONS);
    assert!(enabled(&actions, ContextualActionId::AddColumnLeft));
    assert!(enabled(&actions, ContextualActionId::Delete));
    assert!(enabled(&actions, ContextualActionId::SortAscending));
    assert!(enabled(&actions, ContextualActionId::SortDescending));
    assert!(enabled(&actions, ContextualActionId::RenameColumn));
    assert!(!enabled(&actions, ContextualActionId::RemoveSort));

    let locale = Locale::try_from_str("en").expect("English is a locale");
    document.sort_rows(0, SortDirection::Ascending, &locale);

    assert!(is_enabled(
        &document,
        &selection(&[], &[0]),
        ContextualActionId::RemoveSort
    ));
}

#[test]
fn a_missing_selected_column_disables_the_column_actions() {
    let document = document();
    let live = selection(&[], &[99]);

    assert!(!is_enabled(
        &document,
        &live,
        ContextualActionId::AddColumnLeft
    ));
    assert!(!is_enabled(
        &document,
        &live,
        ContextualActionId::SortAscending
    ));
    assert!(!is_enabled(
        &document,
        &live,
        ContextualActionId::RenameColumn
    ));
}

#[test]
fn a_document_without_a_table_disables_every_action() {
    let document = Document::new(CsvConfiguration::default());
    let live = selection(&[0], &[0]);

    for id in ContextualActionId::ALL {
        assert!(!is_enabled(&document, &live, id), "{id:?} must be disabled");
    }
    assert!(actions_for(&document, &GridSelection::default()).is_empty());
    let actions = actions_for(&document, &live);
    assert_eq!(ids(&actions), ROW_ACTIONS);
    assert!(actions.iter().all(|action| !action.enabled));
}

#[test]
fn every_action_has_a_translated_title() {
    for id in ContextualActionId::ALL {
        for language in Language::ALL {
            if language != Language::System {
                assert!(!tr(language, id.title_key()).is_empty());
            }
        }
    }
}

#[test]
fn the_fold_split_matches_the_ios_examples() {
    let centered = FoldGap::make(FoldRegion::new(460.0, 32.0, 44.0), bar(951.0), 7)
        .expect("a centered fold splits");
    assert_eq!(centered.split, 4);
    assert_eq!(centered.leading, 460.0);
    assert_eq!(centered.width, 32.0);
    assert_eq!(centered.trailing, 459.0);

    let rounded = FoldGap::make(FoldRegion::new(200.0, 20.0, 44.0), bar(600.0), 3)
        .expect("a fold away from the edges splits");
    assert_eq!(rounded.split, 2);
}

#[test]
fn a_fold_at_an_edge_or_inactive_never_splits() {
    assert_eq!(
        FoldGap::make(FoldRegion::new(0.0, 20.0, 44.0), bar(600.0), 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(590.0, 20.0, 44.0), bar(600.0), 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(290.0, 0.0, 44.0), bar(600.0), 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(290.0, 20.0, 44.0), bar(600.0), 1),
        None
    );
}

#[test]
fn a_horizontal_fold_is_ignored() {
    assert!(!FoldRegion::new(290.0, 40.0, 20.0).is_vertical());
    assert_eq!(
        fold_gap(Some(FoldRegion::new(290.0, 40.0, 20.0)), bar(600.0), 7),
        None
    );
    assert!(fold_gap(Some(FoldRegion::new(290.0, 20.0, 44.0)), bar(600.0), 7).is_some());
    assert_eq!(fold_gap(None, bar(600.0), 7), None);
}

#[test]
fn a_fold_near_an_edge_puts_all_actions_on_the_wide_side() {
    let left = FoldGap::make(FoldRegion::new(40.0, 24.0, 44.0), bar(600.0), 7)
        .expect("all actions fit after a near-left fold");
    assert_eq!(left.split, 0);
    assert_eq!(left.leading, 40.0);
    assert_eq!(left.trailing, 536.0);

    let right = FoldGap::make(FoldRegion::new(536.0, 24.0, 44.0), bar(600.0), 7)
        .expect("all actions fit before a near-right fold");
    assert_eq!(right.split, 7);
    assert_eq!(right.trailing, 40.0);
}

#[test]
fn an_infeasible_split_returns_none_instead_of_small_buttons() {
    assert_eq!(
        FoldGap::make(FoldRegion::new(160.0, 32.0, 44.0), bar(360.0), 7),
        None
    );
}

#[test]
fn the_action_bar_keeps_buttons_out_of_the_fold() {
    let actions = column_actions();
    let clicked = Rc::new(RefCell::new(None));
    let state = Rc::clone(&clicked);
    let shown = actions.clone();

    let mut harness = UiHarness::new(Theme::desktop_light(), [951.0, 44.0], move |ui| {
        if let Some(id) = contextual_action_bar(
            ui,
            MobileMetrics::phone(),
            EN,
            &shown,
            Some(FoldRegion::new(460.0, 32.0, 44.0)),
        ) {
            *state.borrow_mut() = Some(id);
        }
    });
    harness.run();

    for action in &actions {
        let rect = harness.get_by_label(tr(EN, action.id.title_key())).rect();
        assert!(
            rect.max.x <= 460.5 || rect.min.x >= 491.5,
            "{:?} overlaps the fold at {rect:?}",
            action.id
        );
    }

    harness.get_by_label(tr(EN, "Delete Selection")).click();
    harness.run();

    assert_eq!(*clicked.borrow(), Some(ContextualActionId::Delete));
}

#[test]
fn a_fold_near_either_edge_keeps_every_button_touch_sized() {
    for (fold, min_x, max_x) in [
        (FoldRegion::new(40.0, 24.0, 44.0), 64.0, f32::INFINITY),
        (FoldRegion::new(536.0, 24.0, 44.0), 0.0, 536.0),
    ] {
        let actions = column_actions();
        let shown = actions.clone();
        let mut harness = UiHarness::new(Theme::desktop_light(), [600.0, 44.0], move |ui| {
            contextual_action_bar(ui, MobileMetrics::phone(), EN, &shown, Some(fold));
        });
        harness.run();

        for action in &actions {
            let rect = harness.get_by_label(tr(EN, action.id.title_key())).rect();
            assert!(
                rect.width() >= 44.0 - 0.5,
                "{:?} is narrower than 44 pt at {rect:?}",
                action.id
            );
            assert!(
                rect.min.x >= min_x - 0.5 && rect.max.x <= max_x + 0.5,
                "{:?} is on the wrong side of the fold at {rect:?}",
                action.id
            );
        }
    }
}

#[test]
fn an_unavoidable_fold_falls_back_to_a_touch_sized_unfolded_bar() {
    let actions = column_actions();
    let shown = actions.clone();
    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        contextual_action_bar(
            ui,
            MobileMetrics::phone(),
            EN,
            &shown,
            Some(FoldRegion::new(160.0, 32.0, 44.0)),
        );
    });
    harness.run();

    for action in &actions {
        let rect = harness.get_by_label(tr(EN, action.id.title_key())).rect();
        assert!(
            rect.width() >= 44.0 - 0.5,
            "{:?} is narrower than 44 pt at {rect:?}",
            action.id
        );
    }
}

#[test]
fn a_disabled_action_cannot_be_run() {
    let actions = [ContextualAction {
        id: ContextualActionId::Delete,
        enabled: false,
    }];
    let clicked = Rc::new(RefCell::new(None));
    let state = Rc::clone(&clicked);

    let mut harness = UiHarness::new(Theme::desktop_light(), [360.0, 44.0], move |ui| {
        if let Some(id) = contextual_action_bar(ui, MobileMetrics::phone(), EN, &actions, None) {
            *state.borrow_mut() = Some(id);
        }
    });
    harness.run();

    harness.get_by_label(tr(EN, "Delete Selection")).click();
    harness.run();

    assert_eq!(*clicked.borrow(), None);
}

fn snapshot(size: [f32; 2], metrics: MobileMetrics, fold: Option<FoldRegion>, name: &str) {
    if !tabtool_ui::harness::render_snapshots_enabled() {
        return;
    }
    let actions = column_actions();
    let mut harness = UiHarness::new(Theme::desktop_light(), size, move |ui| {
        contextual_action_bar(ui, metrics, EN, &actions, fold);
    });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_phone_action_bar_matches_its_snapshot() {
    snapshot(
        [360.0, 800.0],
        MobileMetrics::phone(),
        None,
        "mobile-action-bar-phone",
    );
}

#[test]
fn the_tablet_action_bar_matches_its_snapshot() {
    snapshot(
        [800.0, 1280.0],
        MobileMetrics::tablet(),
        None,
        "mobile-action-bar-tablet",
    );
}

#[test]
fn the_folded_action_bar_matches_its_snapshot() {
    snapshot(
        [951.0, 800.0],
        MobileMetrics::phone(),
        Some(FoldRegion::new(460.0, 32.0, 44.0)),
        "mobile-action-bar-fold",
    );
}

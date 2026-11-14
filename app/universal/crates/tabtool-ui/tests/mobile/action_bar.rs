//! Contextual action derivation, document enablement and fold-aware splitting.

use core::cell::RefCell;
use std::rc::Rc;

use tabtool_core::{CsvConfiguration, Locale, SortDirection};
use tabtool_document::{Document, GridSelection};
use tabtool_i18n::{Language, tr};
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

fn ids(actions: &[ContextualAction]) -> Vec<ContextualActionId> {
    actions.iter().map(|action| action.id).collect()
}

fn enabled(actions: &[ContextualAction], id: ContextualActionId) -> bool {
    actions
        .iter()
        .find(|action| action.id == id)
        .is_some_and(|action| action.enabled)
}

#[test]
fn rows_offer_the_three_row_actions() {
    let selection = GridSelection {
        rows: [1].into(),
        columns: Default::default(),
    };

    assert_eq!(
        actions_for_selection(&selection, true),
        [
            ContextualActionId::AddRowAbove,
            ContextualActionId::AddRowBelow,
            ContextualActionId::Delete,
        ]
    );
}

#[test]
fn columns_offer_the_seven_column_actions() {
    let selection = GridSelection {
        rows: Default::default(),
        columns: [0].into(),
    };

    assert_eq!(
        actions_for_selection(&selection, true),
        [
            ContextualActionId::AddColumnLeft,
            ContextualActionId::AddColumnRight,
            ContextualActionId::Delete,
            ContextualActionId::SortAscending,
            ContextualActionId::SortDescending,
            ContextualActionId::RemoveSort,
            ContextualActionId::RenameColumn,
        ]
    );
}

#[test]
fn nothing_selected_or_no_structural_edits_offers_nothing() {
    assert!(actions_for_selection(&GridSelection::default(), true).is_empty());
    assert!(
        actions_for_selection(
            &GridSelection {
                rows: [1].into(),
                columns: Default::default(),
            },
            false,
        )
        .is_empty()
    );
}

#[test]
fn a_row_selection_enables_the_row_actions() {
    let mut document = document();
    document.selection_mut().rows.insert(0);

    let actions = actions_for(&document);

    assert_eq!(
        ids(&actions),
        [
            ContextualActionId::AddRowAbove,
            ContextualActionId::AddRowBelow,
            ContextualActionId::Delete,
        ]
    );
    assert!(actions.iter().all(|action| action.enabled));
}

#[test]
fn a_column_selection_enables_the_column_actions_and_remove_sort_waits() {
    let mut document = document();
    document.selection_mut().columns.insert(0);

    let actions = actions_for(&document);

    assert_eq!(
        ids(&actions),
        [
            ContextualActionId::AddColumnLeft,
            ContextualActionId::AddColumnRight,
            ContextualActionId::Delete,
            ContextualActionId::SortAscending,
            ContextualActionId::SortDescending,
            ContextualActionId::RemoveSort,
            ContextualActionId::RenameColumn,
        ]
    );
    assert!(enabled(&actions, ContextualActionId::AddColumnLeft));
    assert!(enabled(&actions, ContextualActionId::Delete));
    assert!(enabled(&actions, ContextualActionId::SortAscending));
    assert!(enabled(&actions, ContextualActionId::SortDescending));
    assert!(enabled(&actions, ContextualActionId::RenameColumn));
    assert!(!enabled(&actions, ContextualActionId::RemoveSort));

    let locale = Locale::try_from_str("en").expect("English is a locale");
    document.sort_rows(0, SortDirection::Ascending, &locale);

    assert!(is_enabled(&document, ContextualActionId::RemoveSort));
}

#[test]
fn a_missing_selected_column_disables_the_column_actions() {
    let mut document = document();
    document.selection_mut().columns.insert(99);

    assert!(!is_enabled(&document, ContextualActionId::AddColumnLeft));
    assert!(!is_enabled(&document, ContextualActionId::SortAscending));
    assert!(!is_enabled(&document, ContextualActionId::RenameColumn));
}

#[test]
fn a_document_without_a_table_disables_every_action() {
    let document = Document::new(CsvConfiguration::default());

    for id in ContextualActionId::ALL {
        assert!(!is_enabled(&document, id), "{id:?} must be disabled");
    }
    assert!(actions_for(&document).is_empty());
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
    let centered = FoldGap::make(FoldRegion::new(460.0, 32.0, 44.0), 951.0, 7)
        .expect("a centered fold splits");
    assert_eq!(centered.split, 4);
    assert_eq!(centered.leading, 460.0);
    assert_eq!(centered.width, 32.0);
    assert_eq!(centered.trailing, 459.0);

    let rounded = FoldGap::make(FoldRegion::new(200.0, 20.0, 44.0), 600.0, 3)
        .expect("a fold away from the edges splits");
    assert_eq!(rounded.split, 2);
}

#[test]
fn a_fold_at_an_edge_or_inactive_never_splits() {
    assert_eq!(
        FoldGap::make(FoldRegion::new(0.0, 20.0, 44.0), 600.0, 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(590.0, 20.0, 44.0), 600.0, 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(290.0, 0.0, 44.0), 600.0, 7),
        None
    );
    assert_eq!(
        FoldGap::make(FoldRegion::new(290.0, 20.0, 44.0), 600.0, 1),
        None
    );
}

#[test]
fn a_horizontal_fold_is_ignored() {
    assert!(!FoldRegion::new(290.0, 40.0, 20.0).is_vertical());
    assert_eq!(
        fold_gap(Some(FoldRegion::new(290.0, 40.0, 20.0)), 600.0, 7),
        None
    );
    assert!(fold_gap(Some(FoldRegion::new(290.0, 20.0, 44.0)), 600.0, 7).is_some());
    assert_eq!(fold_gap(None, 600.0, 7), None);
}

#[test]
fn the_action_bar_keeps_buttons_out_of_the_fold() {
    let selection = GridSelection {
        rows: Default::default(),
        columns: [0].into(),
    };
    let actions: Vec<ContextualAction> = actions_for_selection(&selection, true)
        .into_iter()
        .map(|id| ContextualAction { id, enabled: true })
        .collect();
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
    let actions: Vec<ContextualAction> = ContextualActionId::ALL
        .into_iter()
        .filter(|id| {
            !matches!(
                id,
                ContextualActionId::AddRowAbove | ContextualActionId::AddRowBelow
            )
        })
        .map(|id| ContextualAction { id, enabled: true })
        .collect();
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

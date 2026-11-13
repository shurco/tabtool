//! Test-inventory parity with the Apple document tests.
//!
//! This is a checklist, not a mock: every entry names the Swift file and the
//! cases the port claims, the Rust files that carry them, and the Rust test
//! functions that must exist there. A renamed or deleted Rust test fails the
//! run, so the inventory cannot silently drift away from the suite.
//!
//! Cases that are not ported are listed as [`WAIVED`] or [`GAPS`] with a
//! reason. They are never claimed as ported. The Swift side is checked only
//! when the Apple tree is present; the Rust side is always checked.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// One Rust file that carries a ported case.
struct RustTarget {
    /// Path relative to the crate root.
    path: &'static str,
    /// Tests that must exist in that file.
    tests: &'static [&'static str],
}

/// One Apple test file and the ported part of its suite.
struct SwiftFile {
    /// Path relative to `app/apple`.
    file: &'static str,
    /// Swift test functions this file claims are ported.
    cases: &'static [&'static str],
    /// Rust files and tests that carry those cases.
    rust: &'static [RustTarget],
}

/// A case the port deliberately does not carry; `reason` says why.
struct Waived {
    case: &'static str,
    reason: &'static str,
}

/// A difference that is not a UI waiver; `reason` says what is missing.
struct Gap {
    case: &'static str,
    reason: &'static str,
}

/// Rust test files with no direct Apple counterpart.
struct RustOnly {
    path: &'static str,
    reason: &'static str,
}

/// The Swift files named by the Plan 3 tasks that define the document contract.
const PLAN_FILES: &[&str] = &[
    "TabToolTests/CSVDocumentTests.swift",
    "TabToolTests/CSVDocumentConcurrencyTests.swift",
    "TabToolTests/CSVDocumentInsertTests.swift",
    "TabToolTests/CSVDocumentMovingTests.swift",
    "TabToolTests/CSVDocumentFormatTests.swift",
    "TabToolTests/FormatChangePlannerTests.swift",
    "TabToolTests/RevertCoordinatorTests.swift",
    "TabToolTests/CSVDocumentSortingTests.swift",
    "TabToolKit/Tests/TabToolKitTests/CSVClipboardTests.swift",
    "TabToolTests/CSVDocumentRenamingTests.swift",
    "TabToolTests/RowFilterModelTests.swift",
    "TabToolTests/CSVDocumentFilteringTests.swift",
    "TabToolTests/CSVDocumentLargeTests.swift",
    "TabToolTests/CSVDocumentLargeEditingTests.swift",
    "TabToolTests/CSVDocumentLargeSavingTests.swift",
];

/// Swift cases ported by the document layer, mapped to their Rust tests.
const PORTED: &[SwiftFile] = &[
    SwiftFile {
        file: "TabToolTests/CSVDocumentTests.swift",
        cases: &[
            "loadParsesAFileIntoTable",
            "newDocumentIsThreeByOneAndReady",
            "newDocumentUsesTheInjectedConfiguration",
            "newDocumentWithoutHeaderPreferenceHasNoHeaders",
            "newDocumentWithHeaderPreferenceHasMatchingHeaders",
            "loadDetectsFormatWhenConfigurationIsNil",
            "snapshotAndEncodedDataRoundTrip",
            "snapshotBeforeLoadThrowsNotReady",
            "invalidDataProducesReadFailedPhase",
            "ambiguousLoadedContentBlocksSaving",
            "retryingAFailedLoadReadsFromFileURL",
            "retryingAFailedLoadReportsAnUnreadableFile",
            "bomIsPreservedOnSave",
            "needsAttentionSnapshotKeepsThrowingTheTypedError",
        ],
        rust: &[
            RustTarget {
                path: "tests/types.rs",
                tests: &[
                    "a_new_document_starts_loading_with_a_blank_three_by_one_table",
                    "a_new_document_uses_the_injected_configuration",
                    "the_row_source_mirrors_the_table",
                ],
            },
            RustTarget {
                path: "tests/load.rs",
                tests: &[
                    "an_empty_file_loads_as_a_one_by_one_table",
                    "load_uses_a_supplied_configuration",
                    "load_detects_the_format_when_the_configuration_is_default",
                    "invalid_bytes_fail_the_read_and_leave_no_table",
                    "a_missing_file_is_unreadable_not_a_parse_failure",
                    "a_failed_load_can_be_retried_from_its_file",
                    "new_document_is_three_columns_by_one_row_and_ready",
                    "new_document_with_a_header_preference_gets_three_empty_headers",
                ],
            },
            RustTarget {
                path: "tests/snapshot.rs",
                tests: &[
                    "snapshot_before_load_is_not_ready",
                    "snapshot_captures_the_table_configuration_and_bom",
                    "encoded_data_round_trips_through_the_snapshot",
                    "the_utf8_bom_is_written_back_from_the_snapshot",
                    "encoded_data_reports_the_write_issue_for_ambiguous_output",
                ],
            },
            RustTarget {
                path: "tests/saving.rs",
                tests: &[
                    "save_preserves_the_utf8_bom_of_the_loaded_file",
                    "ambiguous_no_quote_output_blocks_save_with_the_typed_error",
                    "unencodable_text_blocks_save_with_the_typed_error",
                    "a_read_failed_document_cannot_save",
                    "can_save_requires_a_path_a_table_and_a_writable_phase",
                ],
            },
            RustTarget {
                path: "src/saving.rs",
                tests: &[
                    "a_loading_document_does_not_publish_readiness",
                    "repeated_snapshot_requests_at_one_revision_reuse_the_cached_arc",
                    "a_revision_bump_invalidates_the_cached_snapshot",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentConcurrencyTests.swift",
        cases: &[
            "loadingANewDocumentIsIdempotent",
            "snapshotOffMainActorReturnsLoadedContent",
            "snapshotOffMainActorSeesCellChange",
            "snapshotOffMainActorFollowsUndo",
            "snapshotOffMainActorThrowsNotReadyBeforeLoad",
            "snapshotOffMainActorThrowsNotReadyAfterReadFailure",
            "serializationOffMainActorMatchesEncodedData",
            "serializationOffMainActorRejectsBlockedSnapshot",
            "encodedDataThrowsWhileNeedsAttention",
        ],
        rust: &[
            RustTarget {
                path: "tests/load.rs",
                tests: &[
                    "a_second_load_does_not_replace_published_content",
                    "a_failed_load_does_not_publish_a_partial_table",
                ],
            },
            RustTarget {
                path: "tests/edit.rs",
                tests: &["a_snapshot_taken_after_a_cell_edit_sees_the_change"],
            },
            RustTarget {
                path: "tests/undo.rs",
                tests: &["a_snapshot_taken_after_undo_sees_the_previous_text"],
            },
            RustTarget {
                path: "tests/snapshot.rs",
                tests: &[
                    "snapshot_before_load_is_not_ready",
                    "snapshot_after_an_unreadable_file_is_not_ready",
                    "encoded_data_round_trips_through_the_snapshot",
                    "encoded_data_reports_the_write_issue_for_ambiguous_output",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentInsertTests.swift",
        cases: &[
            "insertingAfterWithoutASelectionAppendsAtTheEnd",
            "insertingBeforeTheSelectionPlacesTheNewRowAndColumnFirst",
        ],
        rust: &[RustTarget {
            path: "tests/edit.rs",
            tests: &[
                "inserting_before_the_selection_places_the_new_row_and_column_first",
                "inserting_after_without_a_selection_appends_at_the_end",
                "inserting_above_without_a_selection_inserts_at_the_top",
                "inserting_below_the_last_selected_row",
                "inserting_right_of_the_last_selected_column",
                "a_new_row_replaces_the_selection_on_the_row_axis",
                "a_new_column_replaces_the_selection_on_the_column_axis",
                "inserting_a_column_in_header_mode_gives_it_an_empty_header",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentMovingTests.swift",
        cases: &[
            "moveRowsDownKeepsGapsAndSelection",
            "moveColumnsLeftMovesHeadersAndSelection",
            "movesAreDisabledAtEdgesAndWithoutSelection",
            "moveRowsClearsSort",
            "columnMovesKeepTheSort",
        ],
        rust: &[
            RustTarget {
                path: "tests/structural.rs",
                tests: &[
                    "move_rows_down_keeps_gaps_and_selection",
                    "move_columns_left_moves_headers_and_selection",
                    "one_step_moves_are_disabled_at_the_edges",
                    "moving_rows_clamps_the_destination_to_the_end",
                    "moving_columns_clamps_the_destination_to_the_right_edge",
                    "moving_with_an_empty_selection_records_nothing",
                ],
            },
            RustTarget {
                path: "tests/sort.rs",
                tests: &[
                    "structural_change_clears_sort",
                    "column_moves_keep_the_sort_and_remove_sort_restores_row_order",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentFormatTests.swift",
        cases: &[
            "decimalChangeConvertsNumericCellsAndUndoes",
            "newDocumentDecimalAndHeaderTransformTypedTable",
            "headerToggleMovesFirstRowAndUndoes",
            "dirtyReparseRequiresConfirmation",
            "forcedReparseReplacesTableAndUndoes",
            "failedReparseKeepsTableAndConfiguration",
            "structuralOperationsRegisterUndo",
            "pasteInsertsRowsAddsColumnsAndConvertsDecimal",
            "copySelectionUsesVisualColumnOrder",
            "writeIssueReportsEncodingAndAmbiguity",
            "cellUndoAndRedoRestoreValuesStepwise",
            "untitledConfigurationChangeOnlyUpdatesFormat",
            "missingFileReparseReportsUnreadableFile",
            "detectFormatFromDiskReadsWithoutApplying",
            "snapshotKeepsBOMStateAcrossReread",
            "operationsUseDocumentedUndoActionNames",
            "reparseRejectsEditsMadeWhileParsing",
            "overlappingRereadIsRejected",
            "deletingEveryColumnKeepsOneColumn",
            "deleteSelectionKeepsOneColumn",
            "deletingTheOnlyColumnKeepsIt",
        ],
        rust: &[
            RustTarget {
                path: "tests/format.rs",
                tests: &[
                    "decimal_change_converts_numeric_cells_and_undoes",
                    "new_document_decimal_and_header_transform_typed_table",
                    "header_toggle_moves_first_row_and_undoes",
                    "dirty_reparse_requires_confirmation",
                    "forced_reparse_replaces_table_and_undoes",
                    "failed_reparse_keeps_table_and_configuration_without_undo",
                    "missing_file_reparse_reports_cannot_read_file",
                    "untitled_configuration_change_cannot_reread_and_keeps_the_table",
                    "untitled_configuration_change_recomputes_the_write_issue",
                    "reparse_updates_the_utf8_bom_and_keeps_older_snapshots",
                    "detect_format_from_disk_reads_without_applying",
                    "overlapping_reread_is_rejected",
                    "a_stale_candidate_is_rejected_after_a_mutation",
                ],
            },
            RustTarget {
                path: "tests/undo.rs",
                tests: &[
                    "cell_undo_and_redo_restore_the_text",
                    "undo_action_keys_are_stable_english_identifiers",
                    "undo_of_a_delete_restores_the_table_and_the_selection",
                    "undo_of_a_column_move_restores_the_column_order",
                ],
            },
            RustTarget {
                path: "tests/clipboard.rs",
                tests: &[
                    "paste_inserts_rows_adds_columns_and_converts_decimal",
                    "copy_selection_uses_visual_column_order",
                ],
            },
            RustTarget {
                path: "tests/structural.rs",
                tests: &[
                    "deleting_every_selected_column_keeps_the_leftmost_one",
                    "deleting_the_only_column_is_a_no_op",
                ],
            },
            RustTarget {
                path: "src/format.rs",
                tests: &["a_candidate_clears_the_sort_and_undo_restores_sort_and_selection"],
            },
            RustTarget {
                path: "src/undo.rs",
                tests: &["undoing_a_structural_change_restores_the_sort_and_configuration"],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/FormatChangePlannerTests.swift",
        cases: &[
            "onlyDecimalChangesPlanDecimal",
            "onlyHeaderChangesPlanHeader",
            "separatorChangePlansReparse",
            "identicalConfigurationPlansNone",
            "encodingChangePlansReparse",
            "parsingChangeWinsOverHeaderChange",
        ],
        rust: &[RustTarget {
            path: "tests/format_planner.rs",
            tests: &[
                "only_decimal_changes_plan_decimal",
                "only_header_changes_plan_header",
                "separator_change_plans_reparse",
                "identical_configuration_plans_none",
                "encoding_change_plans_reparse",
                "escape_change_plans_reparse",
                "parsing_change_wins_over_header_change",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolTests/RevertCoordinatorTests.swift",
        cases: &[
            "cleanDocumentRevertsWithoutConfirmation",
            "dirtyRevertCanBeCancelledOrConfirmed",
            "revertRequiresAFileBackedDocument",
        ],
        rust: &[
            RustTarget {
                path: "tests/revert.rs",
                tests: &[
                    "clean_document_reverts_without_confirmation",
                    "dirty_revert_can_be_cancelled_or_confirmed",
                    "revert_requires_a_file_backed_document",
                    "revert_uses_the_current_configuration",
                    "revert_leaves_the_file_untouched",
                    "revert_registers_one_undo_action",
                ],
            },
            RustTarget {
                path: "tests/large_editing.rs",
                tests: &["revert_to_saved_routes_to_the_large_revert"],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentSortingTests.swift",
        cases: &[
            "sortThenUndoAndRedoRestoreRowsAndSortState",
            "ascendingDescendingRemoveRestoresSourceOrder",
            "cellEditKeepsSortAndRemoveSortKeepsTheEditedRow",
            "removeSortClearsStateWhenRowsLookIdentical",
            "resortingSameDirectionAfterEditReordersAndKeepsSource",
            "sortingAnotherColumnComposesAndRemoveSortRestoresOriginal",
            "removeSortUndoAndRedo",
            "structuralChangeClearsSort",
            "alreadySortedColumnSetsSortWithoutUndo",
            "cycleSortGoesAscendingDescendingThenRemoves",
            "cycleSortOnAnotherColumnStartsAscending",
        ],
        rust: &[RustTarget {
            path: "tests/sort.rs",
            tests: &[
                "sort_then_undo_and_redo_restore_rows_and_sort_state",
                "ascending_descending_remove_restores_source_order",
                "cell_edit_keeps_sort_and_remove_sort_keeps_the_edited_row",
                "remove_sort_clears_state_when_rows_look_identical",
                "resorting_same_direction_after_edit_reorders_and_keeps_source",
                "sorting_another_column_composes_and_remove_sort_restores_original",
                "remove_sort_undo_and_redo",
                "structural_change_clears_sort",
                "already_sorted_column_sets_sort_without_undo",
                "changing_direction_without_reordering_still_updates_the_arrow",
                "cycle_sort_goes_ascending_descending_then_removes",
                "cycle_sort_on_another_column_starts_ascending",
                "sorting_requires_a_valid_column",
                "sorting_uses_the_requested_locale",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolKit/Tests/TabToolKitTests/CSVClipboardTests.swift",
        cases: &[
            "parseTSVDoesNotTrimOrInterpretQuotes",
            "makeTSVReplacesTabsAndNewlinesInCells",
            "makeTSVLocalizesNumericCells",
            "makeTSVUsesRequestedLocaleForLocalization",
            "makeTSVPreservesEmptyCellsAndRows",
        ],
        rust: &[RustTarget {
            path: "tests/clipboard.rs",
            tests: &[
                "copy_flattens_tabs_and_line_breaks_and_localizes_numbers",
                "copy_uses_the_supplied_decimal_separator",
                "paste_does_not_trim_or_interpret_quotes",
                "paste_preserves_empty_fields_and_missing_trailing_cells",
                "copy_is_capped_at_one_million_cells",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentRenamingTests.swift",
        cases: &[
            "renameIsOneNamedUndoableStep",
            "renameKeepsTheSortAndColumnIdentity",
            "renameToTheSameNameRecordsNothing",
            "emptyNameIsAllowed",
            "renameWithoutHeaderTurnsHeaderOnAndKeepsRows",
            "renameWithoutHeaderExportsHeaderLineFirst",
            "renameWithoutHeaderKeepsTheSort",
            "renameWithoutHeaderToGeneratedNameRecordsNothing",
            "undoAndRedoRestoreHeaderModeRowsAndGeneratedNames",
            "undoRestoresSortAndInsertedRowsAndColumns",
        ],
        rust: &[RustTarget {
            path: "tests/rename.rs",
            tests: &[
                "rename_is_one_named_undoable_step",
                "rename_keeps_the_sort_and_column_identity",
                "rename_to_the_same_name_records_nothing",
                "empty_name_is_allowed",
                "rename_without_header_turns_header_on_and_keeps_rows",
                "rename_without_header_exports_header_line_first",
                "rename_without_header_keeps_the_sort",
                "rename_without_header_to_generated_name_records_nothing",
                "undo_and_redo_restore_header_mode_rows_and_generated_names",
                "undo_restores_sort_and_inserted_rows_and_columns",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolTests/RowFilterModelTests.swift",
        cases: &[
            "startsWithOneEmptyCriterionAndNoFilter",
            "applyAllCombinesEveryCriterionWithAnd",
            "applyCriterionUsesOnlyThatCriterionEvenWhenDisabled",
            "applyUnknownCriterionChangesNothing",
            "applyHonoursTheDecimalMark",
            "clearRemovesTheFilterAndResetsTheDraft",
            "addCriterionInsertsAnEmptyRowAfterTheGivenOne",
            "addCriterionAfterAnUnknownIDAppends",
            "removeCriterionDropsItButAlwaysKeepsOneRow",
            "removeCriterionDoesNotTouchTheAppliedFilter",
            "refreshReevaluatesTheAppliedFilter",
            "refreshWithoutAFilterKeepsEverythingVisible",
            "refreshDropsAppliedCriteriaOfRemovedColumns",
            "refreshClearsTheFilterWhenNoCriterionKeepsItsColumn",
            "refreshDetachesDraftCriteriaFromRemovedColumns",
            "refreshKeepsCriteriaOnTheSamePositionsAfterAReload",
            "refreshKeepsRemappedCriteriaOnTheNextEdit",
            "fillMissingColumnsOnlyTouchesCriteriaWithoutAColumn",
            "applyWithoutAnActiveCriterionLeavesNoFilter",
            "applyingOnlyInactiveCriteriaClearsAnAppliedFilter",
        ],
        rust: &[RustTarget {
            path: "tests/filter_model.rs",
            tests: &[
                "starts_with_one_empty_criterion_and_no_filter",
                "apply_all_combines_every_criterion_with_and",
                "apply_criterion_uses_only_that_criterion_even_when_disabled",
                "apply_unknown_criterion_changes_nothing",
                "apply_honours_the_decimal_mark",
                "clear_removes_the_filter_and_resets_the_draft",
                "add_criterion_inserts_an_empty_row_after_the_given_one",
                "add_criterion_after_an_unknown_id_appends",
                "remove_criterion_drops_it_but_always_keeps_one_row",
                "remove_criterion_does_not_touch_the_applied_filter",
                "refresh_reevaluates_the_applied_filter",
                "refresh_without_a_filter_keeps_everything_visible",
                "refresh_drops_applied_criteria_of_removed_columns",
                "refresh_clears_the_filter_when_no_criterion_keeps_its_column",
                "refresh_detaches_draft_criteria_from_removed_columns",
                "refresh_keeps_criteria_on_the_same_positions_after_a_reload",
                "refresh_keeps_remapped_criteria_on_the_next_edit",
                "fill_missing_columns_only_touches_criteria_without_a_column",
                "apply_without_an_active_criterion_leaves_no_filter",
                "applying_only_inactive_criteria_clears_an_applied_filter",
            ],
        }],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentFilteringTests.swift",
        cases: &[
            "applyAllFiltersEvaluatesTheCurrentTable",
            "applyFilterUsesOnlyTheGivenCriterion",
            "applyingDropsSelectedRowsThatAreHidden",
            "visibleTableKeepsOnlyTheFilteredRowsWithTheirEdits",
            "visibleTableIsTheWholeTableWithoutAFilter",
            "cellEditDoesNotReevaluateTheFilter",
            "undoAndRedoOfACellEditReevaluateTheFilter",
            "sortReevaluatesTheFilterAndUndoRestoresIt",
            "undoOfRowDeletionKeepsVisibleRowsInRange",
            "revertToSavedReevaluatesTheFilter",
            "deletingAColumnDropsItsCriteria",
            "rowStructureIsLockedOnlyWhileFiltering",
            "columnDeletionStaysAvailableWhileFiltering",
            "columnCopyCoversOnlyVisibleRows",
            "largeModeCannotFilter",
            "memoryModeCanFilterOnceLoaded",
            "largeModeCannotToggleTheFilterBar",
        ],
        rust: &[
            RustTarget {
                path: "tests/filter.rs",
                tests: &[
                    "apply_all_filters_evaluates_the_current_table",
                    "apply_filter_uses_only_the_given_criterion",
                    "applying_drops_selected_rows_that_are_hidden",
                    "visible_table_keeps_only_the_filtered_rows_with_their_edits",
                    "visible_table_is_the_whole_table_without_a_filter",
                    "cell_edit_does_not_reevaluate_the_filter",
                    "undo_and_redo_of_a_cell_edit_reevaluate_the_filter",
                    "sort_reevaluates_the_filter_and_undo_restores_it",
                    "undo_of_row_deletion_keeps_visible_rows_in_range",
                    "revert_to_saved_reevaluates_the_filter",
                    "deleting_a_column_drops_its_criteria",
                    "row_structure_is_locked_only_while_filtering",
                    "column_deletion_stays_available_while_filtering",
                    "column_copy_covers_only_visible_rows",
                    "memory_mode_can_filter_once_loaded",
                    "toggle_filter_bar_keeps_the_filter",
                    "filters_are_not_undoable",
                ],
            },
            RustTarget {
                path: "src/document.rs",
                tests: &[
                    "large_documents_cannot_filter_or_toggle_the_bar",
                    "a_loading_document_cannot_filter",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentLargeTests.swift",
        cases: &[
            "opensLargeAndPublishesSource",
            "routesBySizeAgainstTheThreshold",
            "defaultThresholdIsTheDocumentedLargeFileThreshold",
            "smallDocumentStaysInMemory",
            "structuralCommandsAreDisabled",
            "utf16FileGivesReadFailed",
            "missingFileIsUnreadable",
        ],
        rust: &[
            RustTarget {
                path: "tests/large.rs",
                tests: &[
                    "opens_large_and_publishes_the_source",
                    "routes_a_file_at_or_above_the_threshold_to_large_mode",
                    "routes_a_file_below_the_threshold_to_memory",
                    "documented_threshold_is_one_hundred_mebibytes",
                    "small_documents_stay_in_memory",
                    "structural_commands_are_inert_in_large_mode",
                    "utf16_files_report_a_read_failure",
                    "a_missing_large_file_is_unreadable",
                    "the_row_source_is_available_before_the_index_finishes",
                    "an_appended_file_reports_stale_and_blocks_edits",
                ],
            },
            RustTarget {
                path: "src/large.rs",
                tests: &[
                    "a_partial_index_shows_known_rows_and_refuses_edits",
                    "a_same_text_edit_is_a_no_op_while_indexing",
                    "polling_an_in_memory_document_reports_no_large_status",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentLargeEditingTests.swift",
        cases: &[
            "editRegistersUndoAndRedo",
            "editBackToOriginalIsClean",
            "editRejectedWhenTheFileChangedAndRegistersNoUndo",
            "revertDropsOverlayAndUndo",
            "reparseWithDirtyOverlayRequiresConfirmation",
            "reparseToUTF16IsRefusedAndChangesNothing",
            "headerToggleChangesRowMappingAndIsUndoable",
            "undoSurvivesAHeaderToggle",
            "detectFormatReadsTheSample",
            "copiesSelectedRowsAsTSV",
            "copyOverTheCellLimitIsRefused",
            "structuralActionsAreInertButCellsStayEditable",
        ],
        rust: &[
            RustTarget {
                path: "tests/large_editing.rs",
                tests: &[
                    "edit_registers_undo_and_redo",
                    "edit_back_to_original_is_clean",
                    "edit_rejected_when_the_file_changed_and_registers_no_undo",
                    "revert_drops_the_overlay_and_undo",
                    "apply_parsing_configuration_routes_to_the_large_reopen",
                    "reopen_to_utf16_is_refused_and_changes_nothing",
                    "header_toggle_changes_row_mapping_and_is_undoable",
                    "undo_survives_a_header_toggle",
                    "copies_selected_rows_as_tsv",
                    "copy_over_the_cell_limit_is_refused",
                ],
            },
            RustTarget {
                path: "tests/large.rs",
                tests: &[
                    "detect_format_reads_the_head_of_a_large_file",
                    "structural_commands_are_inert_in_large_mode",
                ],
            },
        ],
    },
    SwiftFile {
        file: "TabToolTests/CSVDocumentLargeSavingTests.swift",
        cases: &[
            "saveWritesTheEditsAndAdoptsTheWrittenFile",
            "saveWithoutEditsLeavesTheFileAlone",
            "saveIsRefusedUntilIndexingFinishes",
            "editsAreRefusedWhileSaving",
            "undoAfterSaveRestoresThePreEditValue",
            "saveAsWritesTheNewFileAndKeepsTheOriginal",
            "truncatedFileIsMarkedStale",
            "appendedFileIsMarkedAppended",
        ],
        rust: &[
            RustTarget {
                path: "tests/large_save.rs",
                tests: &[
                    "save_writes_the_edits_and_adopts_the_written_file",
                    "save_without_edits_leaves_the_file_alone",
                    "save_is_refused_until_indexing_finishes",
                    "edits_are_refused_while_saving_and_cells_are_not_editable",
                    "undo_after_save_restores_the_pre_edit_value",
                    "save_as_writes_the_new_file_and_keeps_the_original",
                    "truncated_file_is_marked_stale",
                    "appended_file_is_marked_appended_and_refuses_the_save",
                ],
            },
            RustTarget {
                path: "tests/large_save_job.rs",
                tests: &[
                    "start_is_refused_while_the_index_is_incomplete",
                    "progress_is_reported_only_while_saving",
                    "cancel_before_the_writer_starts_restores_the_original",
                    "cancel_mid_copy_restores_the_original",
                    "a_failed_save_restores_the_edits_and_undo",
                    "the_error_reaches_job_wait_and_the_document",
                ],
            },
            RustTarget {
                path: "src/large_save.rs",
                tests: &[
                    "the_original_mapping_is_released_before_install",
                    "the_indexer_thread_is_joined_before_the_mapping_is_released",
                    "a_post_swap_sync_failure_is_adopted",
                ],
            },
        ],
    },
];

/// Cases that belong to a UI layer: the field editor, AppKit/UIKit views and
/// localized text. The crate exposes state for them but cannot test them.
const WAIVED: &[Waived] = &[
    Waived {
        case: "CSVDocumentTests.documentErrorsHaveLocalizedDescriptions",
        reason: "error descriptions are user-visible, localized text (Plans 4-5); the crate exposes typed enums",
    },
    Waived {
        case: "CSVDocumentConcurrencyTests.newDocumentIsConstructedOffMainActor",
        reason: "main-actor placement is the caller's; this UI-free crate has no actor or thread policy",
    },
    Waived {
        case: "CSVDocumentConcurrencyTests.openedDocumentIsConstructedOffMainActor",
        reason: "main-actor placement is the caller's; this UI-free crate has no actor or thread policy",
    },
    Waived {
        case: "CSVDocumentConcurrencyTests.referenceFileDocumentInitializerPathRunsOffMainActor",
        reason: "the reference-file document initializer is an Apple document-model API; the Rust caller decides its thread",
    },
    Waived {
        case: "CSVDocumentInsertTests.insertingBeforeTheSelectionNeedsASelection",
        reason: "command-availability predicates are UI; the crate's insert methods act only on valid selections",
    },
    Waived {
        case: "CSVDocumentSortingTests.sortCommitsTheFieldEditorFirst",
        reason: "the field editor is the UI's; the caller commits the cell before calling sort_rows",
    },
    Waived {
        case: "CSVDocumentSortingTests.canSortSelectionNeedsExactlyOneColumn",
        reason: "selection-based command gating is UI; sort_rows validates the column index",
    },
    Waived {
        case: "CSVDocumentRenamingTests.selectedColumnRenameNeedsExactlyOneColumn",
        reason: "selection-based command gating is UI; rename_column validates the column index",
    },
    Waived {
        case: "CSVDocumentRenamingTests.renameSelectedColumnAsksTheGridToOpenTheEditor",
        reason: "opening the grid's field editor is AppKit/UIKit work",
    },
    Waived {
        case: "CSVDocumentFilteringTests.visibleTableCommitsTheCellBeingEdited",
        reason: "the field editor is the UI's; export reads the committed table",
    },
    Waived {
        case: "CSVDocumentFilteringTests.changesOfTheAppliedFilterRepublishFromTheDocument",
        reason: "document-change notifications are UI; the filter state itself is tested",
    },
    Waived {
        case: "CSVDocumentFilteringTests.clearFilterCommitsTheCellBeingEdited",
        reason: "the field editor is the UI's; clear_filter resets the model directly",
    },
    Waived {
        case: "CSVDocumentFilteringTests.toggleFilterBarRepublishesTheDocumentAndKeepsTheFilter",
        reason: "document republishing is UI; tests/filter.rs pins that hiding the bar keeps the filter",
    },
    Waived {
        case: "CSVDocumentLargeTests.gridShowsIndexedRowsBeforeIndexingFinishes",
        reason: "the grid harness is UI; tests/large.rs pins that rows are readable before indexing finishes",
    },
    Waived {
        case: "CSVDocumentLargeTests.growingRowCountKeepsScrollAndSelection",
        reason: "scroll position and selection rendering are UI",
    },
    Waived {
        case: "CSVDocumentLargeTests.statusTextShowsIndexingProgress",
        reason: "the status string and its localization are UI; LargeStatus carries the fraction",
    },
    Waived {
        case: "CSVDocumentLargeEditingTests.accessibilityExposesOnlyVisibleRowsOfALargeFile",
        reason: "accessibility elements are UI (AppKit/UIKit and AccessKit)",
    },
    Waived {
        case: "CSVDocumentLargeSavingTests.statusTextShowsSavingProgress",
        reason: "the status string and its localization are UI; large_save_progress carries the fraction",
    },
    Waived {
        case: "CSVDocumentLargeSavingTests.saveErrorsHaveTheirOwnMessages",
        reason: "user-visible error text is localized UI; the crate returns typed LargeFileSaveError values",
    },
    Waived {
        case: "FormatControlBindingsTests.swift",
        reason: "the whole file binds format controls to AppKit/SwiftUI views",
    },
    Waived {
        case: "GridFilteringTests.swift",
        reason: "the whole file exercises the grid's filter bar",
    },
    Waived {
        case: "FilterDisplayTests.swift",
        reason: "the whole file exercises the filter bar's displayed state",
    },
    Waived {
        case: "LargeDocumentWindowTests.swift",
        reason: "the whole file exercises the AppKit large-document window",
    },
];

/// Differences that are not UI work: they must be reported, not hidden.
const GAPS: &[Gap] = &[
    Gap {
        case: "CSVDocumentConcurrencyTests.cancellingBeforeLoadKeepsSuppliedBytes",
        reason: "the Rust load is synchronous and has no cancellation; a caller that has not loaded can still load",
    },
    Gap {
        case: "CSVDocumentConcurrencyTests.cancellingDuringLoadKeepsBytesAndReloads",
        reason: "the Rust load is synchronous; the no-partial-publish invariant is covered by tests/load.rs",
    },
    Gap {
        case: "CSVDocumentConcurrencyTests.snapshotOffMainActorThrowsValidationError",
        reason: "Rust snapshot() is infallible for a loaded table; encoded_data applies the write gate and returns the typed issue",
    },
    Gap {
        case: "CSVDocumentConcurrencyTests.failedDataOnlyLoadKeepsSuppliedBytes",
        reason: "the document does not retain supplied bytes; the caller can call load_from_bytes again with the same bytes",
    },
    Gap {
        case: "CSVDocumentFormatTests.hasUnsavedChangesTracksDiskBytes",
        reason: "dirty is revision-based by design; the document never re-reads the file to compare bytes",
    },
    Gap {
        case: "CSVDocumentFormatTests.asyncDirtyCheckMatchesDisk",
        reason: "dirty is revision-based by design; there is no asynchronous disk comparison",
    },
    Gap {
        case: "FormatChangePlannerTests.quoteStyleChangePlansReparse",
        reason: "plan_format_change handles the quote character, but no dedicated Rust case pins it yet",
    },
];

/// Rust test files that test a seam with no Apple test file.
const RUST_ONLY: &[RustOnly] = &[
    RustOnly {
        path: "tests/storage.rs",
        reason: "Storage is a Rust seam (read + atomic write) behind load and save; the Apple app uses FileManager directly",
    },
    RustOnly {
        path: "tests/parity.rs",
        reason: "this inventory lists the ported Apple document tests and cannot list itself as ported",
    },
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn apple_dir() -> PathBuf {
    // crates/tabtool-document -> universal -> app -> repository root, then
    // app/apple inside the repository.
    crate_dir().join("../../../../app/apple")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} is not readable: {error}", path.display()))
}

fn qualified(file: &str, case: &str) -> String {
    let name = file.rsplit('/').next().unwrap_or(file);
    format!("{name}.{case}")
}

/// Resolves a Swift test file in either Apple test target.
fn swift_test_file(name: &str) -> Option<PathBuf> {
    let apple = apple_dir();
    [
        apple.join("TabToolTests").join(name),
        apple.join("TabToolKit/Tests/TabToolKitTests").join(name),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// Asserts that a waiver or gap entry names a real Swift file or case.
///
/// Every entry points under `app/apple`; none lives outside this checkout, so
/// no absolute path is needed here.
fn assert_swift_case_exists(entry: &str) {
    if entry.ends_with(".swift") {
        assert!(
            swift_test_file(entry).is_some(),
            "{entry} is missing from the Apple test targets"
        );
        return;
    }
    let (file, case) = entry
        .rsplit_once('.')
        .unwrap_or_else(|| panic!("{entry} is not a qualified Swift case"));
    let name = format!("{file}.swift");
    let path = swift_test_file(&name).unwrap_or_else(|| panic!("{name} is missing"));
    let source = read(&path);
    assert!(
        source.contains(&format!("func {case}(")),
        "{}: Swift case {case} is missing",
        path.display()
    );
}

/// Every ported case has its Rust test, and the Swift case exists on disk.
#[test]
fn ported_cases_map_to_existing_rust_tests() {
    let mut files: BTreeSet<&str> = BTreeSet::new();

    for swift in PORTED {
        assert!(files.insert(swift.file), "{} is listed twice", swift.file);
        assert!(
            !swift.cases.is_empty(),
            "{} lists no ported cases",
            swift.file
        );
        for target in swift.rust {
            let path = crate_dir().join(target.path);
            assert!(path.is_file(), "{}: {} is missing", swift.file, target.path);
            let source = read(&path);
            assert!(
                !target.tests.is_empty(),
                "{} lists no tests for {}",
                swift.file,
                target.path
            );
            for test in target.tests {
                assert!(
                    source.contains(&format!("fn {test}(")),
                    "{}: {} does not contain fn {test}()",
                    target.path,
                    swift.file
                );
            }
        }
    }

    let apple = apple_dir();
    if !apple.is_dir() {
        eprintln!(
            "skipping the Swift side of the parity list: {} is absent",
            apple.display()
        );
        return;
    }
    for swift in PORTED {
        let path = apple.join(swift.file);
        assert!(path.is_file(), "{} is missing", path.display());
        let source = read(&path);
        for case in swift.cases {
            assert!(
                source.contains(&format!("func {case}(")),
                "{}: Swift case {case} is missing",
                swift.file
            );
        }
    }
}

/// The inventory covers exactly the Swift files the plan names.
#[test]
fn the_inventory_covers_the_plan_files() {
    let listed: BTreeSet<&str> = PORTED.iter().map(|swift| swift.file).collect();
    let planned: BTreeSet<&str> = PLAN_FILES.iter().copied().collect();
    assert_eq!(
        listed, planned,
        "the ported inventory and the plan's Swift file list differ"
    );
}

/// Waivers and gaps are documented, not silently claimed as parity.
#[test]
fn waivers_and_gaps_are_documented() {
    let claimed: BTreeSet<String> = PORTED
        .iter()
        .flat_map(|swift| {
            swift
                .cases
                .iter()
                .map(move |case| qualified(swift.file, case))
        })
        .collect();
    let mut waived = BTreeSet::new();

    for entry in WAIVED {
        assert!(
            !entry.reason.trim().is_empty(),
            "{} has no waiver reason",
            entry.case
        );
        assert!(waived.insert(entry.case), "{} is waived twice", entry.case);
        assert!(
            !claimed.contains(entry.case),
            "{} is both waived and claimed as ported",
            entry.case
        );
    }
    for entry in GAPS {
        assert!(
            !entry.reason.trim().is_empty(),
            "{} has no gap reason",
            entry.case
        );
        assert!(
            !waived.contains(entry.case),
            "{} is both a waiver and a gap",
            entry.case
        );
        assert!(
            !claimed.contains(entry.case),
            "{} is both a gap and claimed as ported",
            entry.case
        );
    }
}

/// Waived and gap entries must still name real Swift files and cases.
#[test]
fn waived_and_gap_cases_exist_in_the_apple_tree() {
    let apple = apple_dir();
    if !apple.is_dir() {
        eprintln!(
            "skipping the Swift waiver check: {} is absent",
            apple.display()
        );
        return;
    }
    for entry in WAIVED {
        assert_swift_case_exists(entry.case);
    }
    for entry in GAPS {
        assert_swift_case_exists(entry.case);
    }
}

/// Every integration test file is either mapped or explained.
#[test]
fn every_rust_test_file_is_in_the_inventory() {
    let mut listed: BTreeSet<String> = PORTED
        .iter()
        .flat_map(|swift| swift.rust.iter().map(|target| target.path))
        .filter(|path| path.starts_with("tests/"))
        .map(str::to_owned)
        .collect();
    for entry in RUST_ONLY {
        assert!(
            crate_dir().join(entry.path).is_file(),
            "{} is missing",
            entry.path
        );
        assert!(
            !entry.reason.trim().is_empty(),
            "{} has no reason",
            entry.path
        );
        assert!(
            listed.insert(entry.path.to_owned()),
            "{} is listed twice",
            entry.path
        );
    }

    let tests = crate_dir().join("tests");
    let entries = std::fs::read_dir(&tests).expect("the tests directory is readable");
    let mut on_disk = BTreeSet::new();
    for entry in entries {
        let entry = entry.expect("a tests directory entry is readable");
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path.file_name().expect("a file name").to_string_lossy();
            on_disk.insert(format!("tests/{name}"));
        }
    }

    let missing: Vec<&String> = on_disk.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "test files absent from the parity inventory: {missing:?}"
    );
    let stale: Vec<&String> = listed.difference(&on_disk).collect();
    assert!(
        stale.is_empty(),
        "the parity inventory names missing test files: {stale:?}"
    );
}

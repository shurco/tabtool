import Combine
import Foundation
import TabToolKit

/// The document side of the row filter: availability, evaluation against the
/// table, and the row-structure lock that applies while a filter is active.
extension CSVDocument {
    /// Whether the filter bar can be used: in-memory documents only.
    var canFilter: Bool {
        !isLargeMode && table != nil
    }

    /// Whether rows can be added, deleted or moved. A filtered view shows a
    /// subset of the rows, so those edits wait until the filter is cleared.
    var canEditRowStructure: Bool {
        !rowFilter.isFiltering
    }

    /// Whether a row can be inserted through the toolbar and menus.
    var canInsertRows: Bool {
        isEditableContent && canEditRowStructure
    }

    /// The table as shown, for export: only the rows that pass the applied
    /// filter, read after the cell being edited is committed.
    func visibleTableAfterEndingEditing() -> CSVTable? {
        guard var table = tableAfterEndingEditing() else { return nil }
        if let visibleRows = rowFilter.visibleRows {
            table.deleteRows(at: IndexSet(integersIn: 0..<table.rowCount).subtracting(IndexSet(visibleRows)))
        }
        return table
    }

    /// Applies every enabled draft criterion to the current table.
    func applyAllFilters() {
        guard canFilter, let table = tableAfterEndingEditing() else { return }
        rowFilter.applyAll(to: table, decimalMark: configuration.decimalMark)
        keepSelectionVisible()
    }

    /// Applies one draft criterion alone to the current table.
    func applyFilter(criterionID: UUID) {
        guard canFilter, let table = tableAfterEndingEditing() else { return }
        rowFilter.apply(criterionID: criterionID, to: table, decimalMark: configuration.decimalMark)
        keepSelectionVisible()
    }

    /// Removes the applied filter and resets the draft criteria.
    ///
    /// Ends cell editing first, like applying does, so a cell being edited is
    /// committed instead of lost when the grid reloads.
    func clearFilter() {
        endEditing()
        rowFilter.clear()
    }

    /// Re-evaluates the applied filter after the table was replaced or undone.
    ///
    /// Cell edits skip this on purpose, so an edited row stays in view.
    func refreshFilter() {
        guard let table else { return }
        rowFilter.refresh(source: table, decimalMark: configuration.decimalMark)
        keepSelectionVisible()
    }

    /// Shows or hides the filter bar; an applied filter stays active.
    func toggleFilterBar() {
        guard canFilter else { return }
        rowFilter.isBarVisible.toggle()
    }

    /// Makes a change of the visible rows or of the bar visibility republish
    /// the document, so the grid, toolbar and menus that observe it update.
    func observeRowFilter() {
        guard filterObservation == nil else { return }
        let visibleRowsChanges = rowFilter.$visibleRows.dropFirst().map { _ in }
        let barChanges = rowFilter.$isBarVisible.dropFirst().map { _ in }
        filterObservation = visibleRowsChanges.merge(with: barChanges).sink { [weak self] in
            MainActor.assumeIsolated { self?.objectWillChange.send() }
        }
    }

    /// Drops selected rows that the filter hides.
    private func keepSelectionVisible() {
        guard let visible = rowFilter.visibleRows, !selection.rows.isEmpty else { return }
        let rows = selection.rows.intersection(IndexSet(visible))
        guard rows != selection.rows else { return }
        selection = GridSelection(rows: rows, columns: selection.columns)
    }
}

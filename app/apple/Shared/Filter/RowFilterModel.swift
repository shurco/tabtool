import Foundation
import TabToolKit

/// The row filter of one document: the criteria being edited, the filter that
/// is applied, and the rows it lets through.
///
/// The draft `criteria` are edited freely by the filter bar and affect nothing
/// until applied. The applied filter is a snapshot: `visibleRows` is evaluated
/// by an apply or a `refresh`, never by a cell edit, so rows do not vanish
/// while they are typed into. The state is per document and is not undoable.
@MainActor
final class RowFilterModel: ObservableObject {
    /// The filter bar rows. Never empty.
    @Published var criteria = [FilterCriterion()]
    /// Whether the filter bar is shown; hiding it keeps an applied filter.
    @Published var isBarVisible = false
    @Published private(set) var appliedFilter: RowFilter?
    /// Ascending model indexes of the rows that pass `appliedFilter`; `nil`
    /// when no filter is applied.
    @Published private(set) var visibleRows: [Int]?

    /// The columns of the source at the last apply or refresh.
    private var knownColumnIDs: [UUID] = []

    nonisolated init() {}

    var isFiltering: Bool { appliedFilter != nil }

    /// Applies every enabled criterion of the draft (AND).
    func applyAll(to source: some RowSource, decimalMark: String) {
        evaluate(RowFilter(criteria: criteria), in: source, decimalMark: decimalMark)
    }

    /// Applies one draft criterion alone, even when its checkbox is off.
    func apply(criterionID: UUID, to source: some RowSource, decimalMark: String) {
        guard var criterion = criteria.first(where: { $0.id == criterionID }) else { return }
        criterion.isEnabled = true
        evaluate(RowFilter(criteria: [criterion]), in: source, decimalMark: decimalMark)
    }

    /// Removes the applied filter and resets the draft to one empty row.
    func clear() {
        appliedFilter = nil
        visibleRows = nil
        criteria = [FilterCriterion()]
    }

    /// Inserts an empty criterion after `id`, or at the end for an unknown ID.
    func addCriterion(after id: UUID) {
        let index = criteria.firstIndex { $0.id == id }.map { $0 + 1 } ?? criteria.count
        criteria.insert(FilterCriterion(), at: index)
    }

    /// Gives every draft criterion that has no column the column `id`, so a
    /// new row starts on a real column instead of matching nothing.
    func fillMissingColumns(with id: UUID) {
        for index in criteria.indices where criteria[index].columnID == nil {
            criteria[index].columnID = id
        }
    }

    /// Removes a criterion; the last remaining row is reset instead, so the
    /// bar always has a row to edit.
    func removeCriterion(id: UUID) {
        criteria.removeAll { $0.id == id }
        if criteria.isEmpty { criteria = [FilterCriterion()] }
    }

    /// Re-evaluates the applied filter against the current rows and forgets
    /// columns that no longer exist: a draft criterion loses its column, an
    /// applied one is dropped, and the filter ends when none is active.
    ///
    /// A reload gives every column a new identity; that case keeps the
    /// criteria on the columns at the same positions.
    func refresh(source: some RowSource, decimalMark: String) {
        remapReplacedColumns(to: source.columnIDs)
        let columnIDs = Set(source.columnIDs)
        func isMissing(_ criterion: FilterCriterion) -> Bool {
            criterion.columnID.map { !columnIDs.contains($0) } ?? false
        }
        for index in criteria.indices where isMissing(criteria[index]) {
            criteria[index].columnID = nil
        }
        guard var filter = appliedFilter else { return }
        filter.criteria.removeAll(where: isMissing)
        evaluate(filter, in: source, decimalMark: decimalMark)
    }

    /// Moves the criteria to the new columns when none of the known columns
    /// survived and the column count did not change.
    private func remapReplacedColumns(to columnIDs: [UUID]) {
        defer { knownColumnIDs = columnIDs }
        guard !knownColumnIDs.isEmpty, knownColumnIDs.count == columnIDs.count,
              Set(knownColumnIDs).isDisjoint(with: columnIDs) else { return }
        let replacements = Dictionary(uniqueKeysWithValues: zip(knownColumnIDs, columnIDs))
        func remapped(_ criterion: FilterCriterion) -> FilterCriterion {
            var criterion = criterion
            criterion.columnID = criterion.columnID.flatMap { replacements[$0] }
            return criterion
        }
        criteria = criteria.map(remapped)
        if var filter = appliedFilter {
            filter.criteria = filter.criteria.map(remapped)
            appliedFilter = filter
        }
    }

    /// Applies `filter`, or removes the applied filter when no criterion of
    /// it is active, so an empty bar never reports a filtered view.
    private func evaluate(_ filter: RowFilter, in source: some RowSource, decimalMark: String) {
        knownColumnIDs = source.columnIDs
        guard filter.isActive(in: source, decimalMark: decimalMark) else {
            appliedFilter = nil
            visibleRows = nil
            return
        }
        if filter != appliedFilter { appliedFilter = filter }
        visibleRows = filter.matchingRows(in: source, decimalMark: decimalMark)
    }
}

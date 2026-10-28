import SwiftUI
import TabToolKit

/// The filter bar between the grid toolbar and the grid: one row per
/// criterion and a footer with the bar-wide actions.
///
/// The bar edits the draft criteria of `document.rowFilter`; the document
/// applies them. The keyboard shortcuts act on the criterion that has focus,
/// or on the last one.
struct RowFilterBar: View {
    @ObservedObject var document: CSVDocument
    @ObservedObject private var model: RowFilterModel
    @FocusState private var focusedID: UUID?

    init(document: CSVDocument) {
        self.document = document
        _model = ObservedObject(wrappedValue: document.rowFilter)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach($model.criteria) { $criterion in
                FilterCriterionRow(
                    criterion: $criterion,
                    columns: columns,
                    focusedID: $focusedID,
                    onApply: { document.applyFilter(criterionID: criterion.id) },
                    onRemove: { model.removeCriterion(id: criterion.id) },
                    onAdd: { model.addCriterion(after: criterion.id) }
                )
            }
            footer
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.bar)
        .background { shortcutButtons }
        .onExitCommand { model.isBarVisible = false }
        .onAppear { focusedID = model.criteria.first?.id }
        .onChange(of: missingColumnFill, initial: true) { _, id in
            if let id { model.fillMissingColumns(with: id) }
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("filter-bar")
    }

    private var footer: some View {
        HStack(spacing: 10) {
            Button("Hide") { model.isBarVisible = false }
                .accessibilityIdentifier("filter-hide")

            Text("Show ⌘F   Insert ⌘I   Remove ⌘⇧⌫   Apply All ⌘↩   Exit ⎋")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.tail)
                .accessibilityHidden(true)

            Spacer(minLength: 0)

            Button("Clear") { document.clearFilter() }
                .accessibilityIdentifier("filter-clear")

            Button("Apply All") { document.applyAllFilters() }
                .keyboardShortcut(.return, modifiers: .command)
                .accessibilityIdentifier("filter-apply-all")
        }
        .controlSize(.small)
    }

    /// Invisible buttons that carry the shortcuts of the row actions, which
    /// cannot sit on the per-row buttons without clashing with each other.
    private var shortcutButtons: some View {
        HStack {
            Button("Add Criterion") { model.addCriterion(after: shortcutTargetID) }
                .keyboardShortcut("i", modifiers: .command)
            Button("Remove Criterion") { model.removeCriterion(id: shortcutTargetID) }
                .keyboardShortcut(.delete, modifiers: [.command, .shift])
        }
        .opacity(0)
        .frame(width: 0, height: 0)
        .accessibilityHidden(true)
    }

    private var columns: [FilterColumn] {
        document.table.map { FilterColumn.columns(of: $0, configuration: document.configuration) } ?? []
    }

    /// The criterion the keyboard shortcuts act on.
    private var shortcutTargetID: UUID {
        focusedID ?? model.criteria.last?.id ?? UUID()
    }

    /// The column to give to criteria that have none; `nil` when no criterion
    /// needs one. A change re-runs the fill, e.g. after a reload or a clear.
    private var missingColumnFill: UUID? {
        model.criteria.contains { $0.columnID == nil } ? columns.first?.id : nil
    }
}

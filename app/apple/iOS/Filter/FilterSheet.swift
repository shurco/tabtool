import SwiftUI
import TabToolKit

/// The row filter on iOS, presented from the toolbar.
///
/// The sheet edits the draft criteria of `document.rowFilter`; the applied
/// filter is a snapshot, so closing the sheet keeps both the draft and the
/// rows the filter lets through.
struct FilterSheet: View {
    @ObservedObject var document: CSVDocument
    @ObservedObject private var model: RowFilterModel

    init(document: CSVDocument) {
        self.document = document
        _model = ObservedObject(wrappedValue: document.rowFilter)
    }

    var body: some View {
        NavigationStack {
            List {
                ForEach($model.criteria) { $criterion in
                    FilterCriterionCard(
                        criterion: $criterion,
                        columns: columns,
                        onRemove: { model.removeCriterion(id: criterion.id) }
                    )
                }
            }
            .navigationTitle("Filters")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { model.isBarVisible = false }
                        .accessibilityIdentifier("filter-hide")
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button {
                        model.addCriterion(after: model.criteria.last?.id ?? UUID())
                    } label: {
                        Image(systemName: "plus")
                    }
                    .keyboardShortcut("i", modifiers: .command)
                    .accessibilityLabel("Add Criterion")
                    .accessibilityIdentifier("filter-add")
                }
            }
            .safeAreaInset(edge: .bottom) {
                footer
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .accessibilityIdentifier("filter-bar")
    }

    private var footer: some View {
        HStack(spacing: 12) {
            Button("Clear") { document.clearFilter() }
                .accessibilityIdentifier("filter-clear")

            Spacer(minLength: 0)

            Text(DocumentStatusBar.rowsText(total: totalRows, visible: model.visibleRows?.count))
                .font(.callout)
                .foregroundStyle(.secondary)

            Spacer(minLength: 0)

            Button("Apply") { document.applyAllFilters() }
                .keyboardShortcut(.return, modifiers: .command)
                .disabled(model.criteria.contains { $0.isEnabled && !$0.isValid })
                .accessibilityIdentifier("filter-apply-all")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(.bar)
    }

    private var columns: [FilterColumn] {
        document.table.map { FilterColumn.columns(of: $0, configuration: document.configuration) } ?? []
    }

    private var totalRows: Int {
        document.rowSource?.rowCount ?? 0
    }
}

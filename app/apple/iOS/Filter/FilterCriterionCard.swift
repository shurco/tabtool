import SwiftUI
import TabToolKit

/// One criterion of the filter sheet: enable, column, operator and value.
///
/// The card edits the draft criterion of `document.rowFilter` only; the sheet
/// applies the draft. Swiping the row removes the criterion.
struct FilterCriterionCard: View {
    @Binding var criterion: FilterCriterion
    let columns: [FilterColumn]
    let onRemove: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 12) {
                Toggle("Enable Criterion", isOn: $criterion.isEnabled)
                    .labelsHidden()
                    .accessibilityIdentifier("filter-enabled")

                Menu {
                    ForEach(columns) { column in
                        Button {
                            criterion.columnID = column.id
                        } label: {
                            Text(verbatim: column.name)
                        }
                    }
                } label: {
                    columnMenuLabel
                        .lineLimit(1)
                }
                .accessibilityIdentifier("filter-column")

                Spacer(minLength: 0)
            }

            HStack(spacing: 8) {
                Menu {
                    ForEach(FilterOperator.allCases, id: \.self) { filterOperator in
                        Button {
                            criterion.op = filterOperator
                        } label: {
                            Text(LocalizedStringKey(filterOperator.title))
                        }
                    }
                } label: {
                    Text(LocalizedStringKey(criterion.op.title))
                }
                .accessibilityIdentifier("filter-operator")

                TextField("Value", text: $criterion.value)
                    .textFieldStyle(.roundedBorder)
                    .disabled(!criterion.op.needsValue)
                    .overlay {
                        if !criterion.isValid {
                            RoundedRectangle(cornerRadius: 5).stroke(.red, lineWidth: 1.5)
                        }
                    }
                    .accessibilityIdentifier("filter-value")
            }

            if !criterion.isValid {
                Text("Invalid regular expression")
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .contain)
        .swipeActions {
            Button(role: .destructive, action: onRemove) {
                Label("Remove", systemImage: "trash")
            }
            .accessibilityIdentifier("filter-remove")
        }
    }

    /// The selected column name, or a prompt before one is chosen.
    private var columnMenuLabel: Text {
        guard let id = criterion.columnID,
              let column = columns.first(where: { $0.id == id }) else {
            return Text("Choose Column")
        }
        return Text(verbatim: column.name)
    }
}

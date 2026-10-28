import SwiftUI
import TabToolKit

/// One criterion of the filter bar: checkbox, column, operator, value and the
/// row actions. The row edits the draft only; `onApply` applies it alone.
struct FilterCriterionRow: View {
    @Binding var criterion: FilterCriterion
    let columns: [FilterColumn]
    var focusedID: FocusState<UUID?>.Binding
    let onApply: () -> Void
    let onRemove: () -> Void
    let onAdd: () -> Void

    var body: some View {
        HStack(spacing: 6) {
            Toggle("Enable Criterion", isOn: $criterion.isEnabled)
                .toggleStyle(.checkbox)
                .labelsHidden()
                .accessibilityIdentifier("filter-enabled")

            Picker("Column", selection: $criterion.columnID) {
                ForEach(columns) { column in
                    Text(verbatim: column.name).tag(Optional(column.id))
                }
            }
            .labelsHidden()
            .frame(width: 130)
            .accessibilityIdentifier("filter-column")

            Picker("Operator", selection: $criterion.op) {
                ForEach(FilterOperator.allCases, id: \.self) { filterOperator in
                    Text(LocalizedStringKey(filterOperator.title)).tag(filterOperator)
                }
            }
            .labelsHidden()
            .frame(width: 132)
            .accessibilityIdentifier("filter-operator")

            TextField("Value", text: $criterion.value)
                .textFieldStyle(.roundedBorder)
                .focused(focusedID, equals: criterion.id)
                .disabled(!criterion.op.needsValue)
                .overlay {
                    if !criterion.isValid {
                        RoundedRectangle(cornerRadius: 5).stroke(.red, lineWidth: 1.5)
                    }
                }
                .help(criterion.isValid ? Text(verbatim: "") : Text("Invalid regular expression"))
                .onSubmit(onApply)
                .accessibilityIdentifier("filter-value")

            Button("Apply", action: onApply)
                .accessibilityIdentifier("filter-apply")

            Button(action: onRemove) {
                rowActionIcon("Remove Criterion", systemImage: "minus")
            }
            .help("Remove Criterion")
            .accessibilityIdentifier("filter-remove")

            Button(action: onAdd) {
                rowActionIcon("Add Criterion", systemImage: "plus")
            }
            .help("Add Criterion")
            .accessibilityIdentifier("filter-add")
        }
        .controlSize(.small)
    }

    /// A fixed frame keeps the − and + buttons the same size; a bare minus
    /// glyph is so flat that its button would shrink.
    private func rowActionIcon(_ title: LocalizedStringKey, systemImage: String) -> some View {
        Label(title, systemImage: systemImage)
            .labelStyle(.iconOnly)
            .frame(width: 12, height: 14)
    }
}

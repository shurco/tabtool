import SwiftUI

/// The iOS Settings screen: one scrollable form with a section per group.
struct IOSSettingsView: View {
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section("General") { GeneralSettings() }
                Section { NewDocumentSettings() } header: { Text("New Document") }
                Section("Grid") { GridSettings() }
                Section("Large Files") { LargeFileSettings() }
                Section {
                    NavigationLink("About") { AboutView() }
                }
            }
            .navigationTitle("Settings")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
        }
    }
}

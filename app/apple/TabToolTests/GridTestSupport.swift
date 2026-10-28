import AppKit
import Foundation
import TabToolKit
@testable import TabTool

/// Shared setup for grid interaction tests.
@MainActor
enum GridTestSupport {
    /// A new document with the main-actor load applied.
    static func makeLoadedNewDocument() async -> CSVDocument {
        let document = CSVDocument(newDocumentConfiguration: { CSVConfiguration() })
        await document.load()
        return document
    }

    /// Attaches a coordinator to a standalone table view built from `document`.
    static func makeGrid(
        _ document: CSVDocument
    ) -> (CSVGridCoordinator, CSVTableView, CSVTableHeaderView) {
        let coordinator = CSVGridCoordinator(document: document)
        let tableView = CSVTableView()
        let headerView = CSVTableHeaderView()
        tableView.headerView = headerView
        tableView.allowsMultipleSelection = true
        coordinator.attach(to: tableView)
        update(coordinator, from: document)
        return (coordinator, tableView, headerView)
    }

    static func update(_ coordinator: CSVGridCoordinator, from document: CSVDocument) {
        coordinator.update(
            presenter: GridPresenter(table: document.table ?? .newDocument(),
                                     configuration: document.configuration,
                                     visibleRows: document.rowFilter.visibleRows),
            isEditable: true
        )
    }

    static func makePasteboard() -> NSPasteboard {
        NSPasteboard(name: NSPasteboard.Name("GridInteractionTests-\(UUID().uuidString)"))
    }
}

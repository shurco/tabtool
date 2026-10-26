import AppKit
import XCTest

final class TabToolUITests: XCTestCase {
    @MainActor
    func testLaunchStartsTheApp() {
        let app = XCUIApplication()
        app.launch()
        XCTAssertEqual(app.state, .runningForeground)
    }

    @MainActor
    func testNewDocumentShowsGrid() {
        let app = XCUIApplication()
        app.launch()
        // A windowless document app may not receive Cmd-N, so use the menu
        // command to create the untitled document.
        app.menuBars.menuItems["New"].click()
        XCTAssertTrue(app.tables["grid"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testAddRowAndColumnButtonsChangeGrid() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let addRow = app.buttons["add-row"]
        let addColumn = app.buttons["add-column"]
        let delete = app.buttons["delete-selection"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        XCTAssertTrue(addColumn.waitForExistence(timeout: 5))
        XCTAssertTrue(delete.exists)
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 3)
        XCTAssertFalse(delete.isEnabled)

        addRow.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)
        XCTAssertTrue(delete.isEnabled)

        addColumn.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 4)
        XCTAssertTrue(delete.isEnabled)
        XCTAssertTrue(grid.exists)
    }

    @MainActor
    func testToolbarContextMenusAddAboveAndLeft() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let addRow = app.buttons["add-row"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        addRow.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()

        // The Table menu now exposes commands with the same titles, so scope
        // the query to the context menu attached to the document window.
        let contextMenu = app.windows.firstMatch.menus.firstMatch
        let addAbove = contextMenu.menuItems["Add Row Above"]
        XCTAssertTrue(addAbove.waitForExistence(timeout: 5))
        addAbove.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)

        let addColumn = app.buttons["add-column"]
        addColumn.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()
        let addLeft = app.windows.firstMatch.menus.firstMatch.menuItems["Add Column Left"]
        XCTAssertTrue(addLeft.waitForExistence(timeout: 5))
        addLeft.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 4)
    }

    @MainActor
    func testCopyPasteThroughEditMenu() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let cell = grid.descendants(matching: .cell).element(boundBy: 0)
        XCTAssertTrue(cell.waitForExistence(timeout: 5))
        cell.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

        app.menuBars.menuItems["Copy"].click()
        app.menuBars.menuItems["Paste"].click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)
    }

    @MainActor
    func testDeleteIsDisabledForTheOnlyColumn() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))
        let delete = app.buttons["delete-selection"]

        // Remove two of the three new-document columns.
        for header in ["C", "B"] {
            let button = grid.buttons[header]
            XCTAssertTrue(button.waitForExistence(timeout: 5))
            button.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
            XCTAssertTrue(delete.isEnabled)
            delete.click()
        }
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 1)

        // The only column stays selected but cannot be deleted.
        let onlyHeader = grid.buttons["A"]
        XCTAssertTrue(onlyHeader.waitForExistence(timeout: 5))
        onlyHeader.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        XCTAssertFalse(delete.isEnabled)
    }

    @MainActor
    func testFormatBarShowsControlsAndStatus() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        for identifier in ["format-encoding", "format-separator", "format-quote",
                           "format-decimal", "format-header"] {
            let control = app.descendants(matching: .any)[identifier].firstMatch
            XCTAssertTrue(control.waitForExistence(timeout: 5),
                          "Missing format control \(identifier)")
        }

        XCTAssertEqual(app.popUpButtons["format-encoding"].value as? String,
                       "Unicode (UTF-8)")
        XCTAssertEqual(app.popUpButtons["format-separator"].value as? String,
                       "Comma (,)")
        XCTAssertEqual(app.popUpButtons["format-quote"].value as? String,
                       "Doubled")
        XCTAssertEqual(app.popUpButtons["format-decimal"].value as? String,
                       "Dot (.)")
        XCTAssertTrue(app.checkBoxes["format-header"].isEnabled)

        let status = app.staticTexts["status-text"]
        XCTAssertTrue(status.waitForExistence(timeout: 5))
        let summary = status.value as? String ?? ""
        XCTAssertTrue(summary.contains("3 columns"))
        XCTAssertTrue(summary.contains("Unicode (UTF-8)"))
        XCTAssertTrue(summary.contains("Comma"))

        // Separator changes on an untitled document update the configuration
        // in place and are reflected by the status line.
        app.popUpButtons["format-separator"].click()
        let semicolon = app.menuItems["Semicolon (;)"]
        XCTAssertTrue(semicolon.waitForExistence(timeout: 5))
        semicolon.click()
        XCTAssertEqual(app.popUpButtons["format-separator"].value as? String,
                       "Semicolon (;)")
        XCTAssertTrue((status.value as? String ?? "").contains("Semicolon"))

        // Toggling Header moves the only row across the header boundary.
        let header = app.checkBoxes["format-header"]
        header.click()
        XCTAssertTrue((status.value as? String ?? "").contains("0 rows"))
        header.click()
        XCTAssertTrue((status.value as? String ?? "").contains("1 row"))
    }

    @MainActor
    func testTableMenuAddsAndDeletesRows() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["Table"].click()
        let addRow = app.menuItems["Add Row Below"]
        XCTAssertTrue(addRow.waitForExistence(timeout: 5))
        XCTAssertTrue(addRow.isEnabled)
        addRow.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)

        app.menuBars.menuBarItems["Table"].click()
        let deleteRows = app.menuItems["Delete Row(s)"]
        XCTAssertTrue(deleteRows.waitForExistence(timeout: 5))
        XCTAssertTrue(deleteRows.isEnabled)
        deleteRows.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)

        let header = grid.buttons["B"]
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()

        app.menuBars.menuBarItems["Table"].click()
        let deleteColumns = app.menuItems["Delete Column(s)"]
        XCTAssertTrue(deleteColumns.waitForExistence(timeout: 5))
        XCTAssertTrue(deleteColumns.isEnabled)
        deleteColumns.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 2)

        app.menuBars.menuBarItems["View"].click()
        XCTAssertTrue(app.menuItems["Show Format Bar"].waitForExistence(timeout: 5))

        app.menuBars.menuBarItems["File"].click()
        let export = app.menuItems["Export…"]
        XCTAssertTrue(export.waitForExistence(timeout: 5))
        XCTAssertTrue(export.isEnabled)
    }

    @MainActor
    func testNeedsAttentionBannerOffersUndo() {
        let app = XCUIApplication()
        app.launch()
        defer {
            NSPasteboard.general.clearContents()
            // Leave the shared format-bar preference visible for later tests.
            if !app.popUpButtons["format-encoding"].exists {
                app.menuBars.menuBarItems["View"].click()
                if app.menuItems["Show Format Bar"].waitForExistence(timeout: 2) {
                    app.menuItems["Show Format Bar"].click()
                }
            }
        }
        app.menuBars.menuItems["New"].click()

        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        app.popUpButtons["format-encoding"].click()
        let latin1 = app.menuItems["Western (Windows Latin 1)"]
        XCTAssertTrue(latin1.waitForExistence(timeout: 5))
        latin1.click()

        // Hide the bar so "Show Format Controls" has something to reveal.
        app.menuBars.menuBarItems["View"].click()
        app.menuItems["Show Format Bar"].click()
        XCTAssertFalse(app.popUpButtons["format-encoding"].waitForExistence(timeout: 1))

        NSPasteboard.general.clearContents()
        XCTAssertTrue(NSPasteboard.general.setString("→", forType: .string))

        let cell = grid.descendants(matching: .cell).element(boundBy: 0)
        XCTAssertTrue(cell.waitForExistence(timeout: 5))
        cell.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        app.menuBars.menuItems["Paste"].click()

        let banner = app.descendants(matching: .any)["needs-attention-banner"].firstMatch
        XCTAssertTrue(banner.waitForExistence(timeout: 5))

        let showControls = app.buttons["show-format-controls"]
        XCTAssertTrue(showControls.waitForExistence(timeout: 5))
        showControls.click()
        XCTAssertTrue(app.popUpButtons["format-encoding"].waitForExistence(timeout: 5))

        let undo = app.buttons["undo-last-edit"]
        XCTAssertTrue(undo.waitForExistence(timeout: 5))
        XCTAssertTrue(undo.isEnabled)
        undo.click()
        XCTAssertFalse(banner.waitForExistence(timeout: 1))
    }

    @MainActor
    func testAboutShowsNoticeAndHelpLink() {
        let app = XCUIApplication()
        app.launch()

        app.menuBars.menuBarItems["TabTool"].click()
        let about = app.menuBars.menuItems["About TabTool"].firstMatch
        XCTAssertTrue(about.waitForExistence(timeout: 5))
        about.click()

        XCTAssertTrue(app.staticTexts["About This App"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.links["Project Repository"].exists)
        XCTAssertTrue(app.links["MIT License"].exists)
        let license = app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS %@ OR value CONTAINS %@",
                        "MIT License", "MIT License")
        ).firstMatch
        XCTAssertTrue(license.exists)

        app.menuBars.menuBarItems["Help"].click()
        XCTAssertTrue(app.menuItems["Help"].waitForExistence(timeout: 5))
    }

    @MainActor
    func testHeaderClickSelectsAColumn() {
        let app = XCUIApplication()
        app.launch()
        app.menuBars.menuItems["New"].click()
        let grid = app.tables["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 5))

        let delete = app.buttons["delete-selection"]
        XCTAssertFalse(delete.isEnabled)

        let header = grid.buttons["B"]
        XCTAssertTrue(header.waitForExistence(timeout: 5))
        header.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).click()
        XCTAssertTrue(delete.isEnabled)

        // A column selection deletes a column; a row selection would remove
        // the only data row instead.
        delete.click()
        XCTAssertEqual(grid.descendants(matching: .tableColumn).count, 2)
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 1)
    }
}

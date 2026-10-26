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

        let addAbove = app.menuItems["Add Row Above"]
        XCTAssertTrue(addAbove.waitForExistence(timeout: 5))
        addAbove.click()
        XCTAssertEqual(grid.descendants(matching: .tableRow).count, 2)

        let addColumn = app.buttons["add-column"]
        addColumn.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).rightClick()
        let addLeft = app.menuItems["Add Column Left"]
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

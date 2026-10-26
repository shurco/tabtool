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
}

import XCTest

/// The iOS document flow: create a document from the library and land in the
/// editor. Runs on every iPhone and iPad simulator, including iPhone Duo.
final class DocumentFlowTests: XCTestCase {
    override func setUp() {
        continueAfterFailure = false
    }

    func testCreateDocumentOpensEditor() {
        let app = XCUIApplication()
        app.launch()

        let addButton = app.buttons["add-document"]
        XCTAssertTrue(addButton.waitForExistence(timeout: 10))
        addButton.tap()

        let newDocument = app.buttons["new-document"]
        XCTAssertTrue(newDocument.waitForExistence(timeout: 5))
        newDocument.tap()

        let grid = app.collectionViews["grid"]
        XCTAssertTrue(grid.waitForExistence(timeout: 15))
        XCTAssertTrue(app.buttons["undo"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["filter-toggle"].waitForExistence(timeout: 5))
    }
}

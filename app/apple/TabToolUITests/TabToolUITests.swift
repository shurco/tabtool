import XCTest

final class TabToolUITests: XCTestCase {
    @MainActor
    func testLaunchStartsTheApp() {
        let app = XCUIApplication()
        app.launch()
        XCTAssertEqual(app.state, .runningForeground)
    }
}

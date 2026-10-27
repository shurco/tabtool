import Testing
@testable import TabToolKit

@Suite struct CellOverlayTests {
    @Test func setReadRemove() {
        let overlay = CellOverlay()
        let key = CellKey(row: 3, column: 1)
        #expect(overlay.isEmpty)
        #expect(overlay[key] == nil)
        overlay.set("x", for: key)
        #expect(overlay[key] == "x")
        #expect(overlay.count == 1)
        #expect(overlay.copy() == [key: "x"])
        overlay.remove(key)
        #expect(overlay.isEmpty)
    }

    @Test func removeAllClears() {
        let overlay = CellOverlay()
        overlay.set("a", for: CellKey(row: 0, column: 0))
        overlay.set("b", for: CellKey(row: 1, column: 0))
        overlay.removeAll()
        #expect(overlay.isEmpty)
    }

    @Test func copyIsASnapshot() {
        let overlay = CellOverlay()
        let key = CellKey(row: 0, column: 0)
        overlay.set("a", for: key)
        let snapshot = overlay.copy()
        overlay.set("b", for: key)
        #expect(snapshot[key] == "a")
    }
}

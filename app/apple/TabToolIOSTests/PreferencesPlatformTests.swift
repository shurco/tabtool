import Testing
@testable import TabTool

@Suite struct PreferencesPlatformTests {
    @Test func iOSOffersTouchRowHeights() {
        #expect(Preferences.rowHeights == [32, 44, 56])
        #expect(Preferences.Default.rowHeight == 44)
        #expect(Preferences.validRowHeight(24) == 44)
    }
}

import Testing
@testable import TabTool

@Suite struct PreferencesPlatformTests {
    @Test func macOSOffersDesktopRowHeights() {
        #expect(Preferences.rowHeights == [20, 24, 32])
        #expect(Preferences.Default.rowHeight == 24)
        #expect(Preferences.validRowHeight(44) == 24)
    }
}

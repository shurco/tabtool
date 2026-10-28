import Foundation
import Testing
import TabToolKit
@testable import TabTool

@Suite struct PreferencesTests {
    @Test func keyStringsArePersistedData() {
        #expect(Preferences.Key.showFormatBar == "showFormatBar")
        #expect(Preferences.Key.appLanguage == "appLanguage")
        #expect(Preferences.Key.newDocumentEncoding == "newDocumentEncoding")
        #expect(Preferences.Key.newDocumentSeparator == "newDocumentSeparator")
        #expect(Preferences.Key.newDocumentQuoteStyle == "newDocumentQuoteStyle")
        #expect(Preferences.Key.newDocumentDecimalMark == "newDocumentDecimalMark")
        #expect(Preferences.Key.newDocumentFirstRowAsHeader == "newDocumentFirstRowAsHeader")
        #expect(Preferences.Key.gridAlternatingRows == "gridAlternatingRows")
        #expect(Preferences.Key.gridRowHeight == "gridRowHeight")
        #expect(Preferences.Key.largeFileThresholdMiB == "largeFileThresholdMiB")
    }

    @Test func emptyDefaultsGiveTheDocumentedDefaults() {
        let store = TestDefaults()
        #expect(Preferences.newDocumentConfiguration(store.defaults) == CSVConfiguration())
        #expect(Preferences.gridAlternatingRows(store.defaults) == true)
        #expect(Preferences.gridRowHeight(store.defaults) == 24)
        #expect(Preferences.largeFileThresholdBytes(store.defaults) == LargeFile.thresholdBytes)
    }

    @Test func storedValuesAreRead() {
        let store = TestDefaults()
        let defaults = store.defaults
        defaults.set(Int(String.Encoding.windowsCP1251.rawValue), forKey: Preferences.Key.newDocumentEncoding)
        defaults.set(";", forKey: Preferences.Key.newDocumentSeparator)
        defaults.set("backslash", forKey: Preferences.Key.newDocumentQuoteStyle)
        defaults.set(",", forKey: Preferences.Key.newDocumentDecimalMark)
        defaults.set(true, forKey: Preferences.Key.newDocumentFirstRowAsHeader)
        defaults.set(32.0, forKey: Preferences.Key.gridRowHeight)
        defaults.set(250, forKey: Preferences.Key.largeFileThresholdMiB)
        defaults.set(false, forKey: Preferences.Key.gridAlternatingRows)

        let configuration = Preferences.newDocumentConfiguration(defaults)
        #expect(configuration.encoding == .windowsCP1251)
        #expect(configuration.columnSeparator == ";")
        #expect(configuration.quoteCharacter == "\"")
        #expect(configuration.escapeCharacter == "\\")
        #expect(configuration.decimalMark == ",")
        #expect(configuration.firstRowAsHeader == true)
        #expect(Preferences.gridRowHeight(defaults) == 32)
        #expect(Preferences.largeFileThresholdBytes(defaults) == 250 * 1_048_576)
        #expect(Preferences.gridAlternatingRows(defaults) == false)
    }

    @Test func unknownStoredValuesFallBackToDefaults() {
        let store = TestDefaults()
        let defaults = store.defaults
        defaults.set(999_999, forKey: Preferences.Key.newDocumentEncoding)
        defaults.set("x", forKey: Preferences.Key.newDocumentSeparator)
        defaults.set("x", forKey: Preferences.Key.newDocumentQuoteStyle)
        defaults.set("x", forKey: Preferences.Key.newDocumentDecimalMark)
        #expect(Preferences.newDocumentConfiguration(defaults) == CSVConfiguration())

        defaults.set(7.0, forKey: Preferences.Key.gridRowHeight)
        #expect(Preferences.gridRowHeight(defaults) == 24)

        for invalid in [0, -5, 7] {
            defaults.set(invalid, forKey: Preferences.Key.largeFileThresholdMiB)
            #expect(Preferences.largeFileThresholdBytes(defaults) == 100 * 1_048_576)
        }

        defaults.set("no", forKey: Preferences.Key.gridAlternatingRows)
        #expect(Preferences.gridAlternatingRows(defaults) == true)
    }

    @Test func defaultThresholdMatchesTheCoreConstant() {
        #expect(Int64(Preferences.Default.largeFileThresholdMiB) * 1_048_576 == LargeFile.thresholdBytes)
    }
}

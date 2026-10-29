import SwiftUI
import Testing
import TabToolKit
@testable import TabTool

@MainActor @Suite struct FormatControlBindingsTests {
    private func bindings(_ configuration: CSVConfiguration = CSVConfiguration(),
                          into received: @escaping (CSVConfiguration) -> Void = { _ in })
        -> FormatControlBindings {
        FormatControlBindings(configuration: configuration, onChange: received)
    }

    @Test func bindingWritesThroughACompleteConfiguration() {
        var received: CSVConfiguration?
        let bindings = bindings { received = $0 }
        bindings.binding(\.columnSeparator).wrappedValue = ";"
        #expect(received?.columnSeparator == ";")
        #expect(received?.encoding == CSVConfiguration().encoding)
    }

    @Test func quoteStylesMapToQuoteAndEscapeCharacters() {
        for (style, quote, escape) in [(QuoteStyle.doubled, "\"", "\""),
                                       (QuoteStyle.backslash, "\"", "\\"),
                                       (QuoteStyle.none, "", "")] {
            var received: CSVConfiguration?
            let bindings = bindings { received = $0 }
            bindings.quoteStyle.wrappedValue = style
            #expect(received?.quoteCharacter == quote)
            #expect(received?.escapeCharacter == escape)
        }
    }
}

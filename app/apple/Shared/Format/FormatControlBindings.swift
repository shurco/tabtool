import SwiftUI
import TabToolKit

/// Bridges a configuration snapshot to bindings that report complete values.
///
/// Every edit hands the caller a whole `CSVConfiguration`, so the document
/// view can derive the operation with `FormatChangePlanner`.
struct FormatControlBindings {
    let configuration: CSVConfiguration
    let onChange: (CSVConfiguration) -> Void

    func binding<Value>(
        _ keyPath: WritableKeyPath<CSVConfiguration, Value>
    ) -> Binding<Value> {
        Binding(
            get: { configuration[keyPath: keyPath] },
            set: { newValue in
                var configuration = configuration
                configuration[keyPath: keyPath] = newValue
                onChange(configuration)
            }
        )
    }

    var quoteStyle: Binding<QuoteStyle> {
        Binding(
            get: { QuoteStyle(configuration: configuration) },
            set: { style in
                var configuration = configuration
                configuration.quoteCharacter = style.quoteCharacter
                configuration.escapeCharacter = style.escapeCharacter
                onChange(configuration)
            }
        )
    }
}

import Foundation
import Testing
@testable import TabToolKit

@Suite struct CSVErrorTests {
    @Test func everyErrorHasAnEnglishDescriptionAndRecoverySuggestion() {
        let errors: [CSVError] = [
            .cannotDecodeData,
            .malformedQuotedField,
            .quoteInUnquotedField,
            .unencodableText,
            .ambiguousOutput
        ]

        for error in errors {
            #expect(error.errorDescription?.isEmpty == false,
                    "Missing description for \(error)")
            #expect(error.recoverySuggestion?.isEmpty == false,
                    "Missing recovery suggestion for \(error)")
        }
    }

    @Test func saveErrorsSuggestUndoOrExportInsteadOfDiscardingEdits() throws {
        let unencodable = try #require(CSVError.unencodableText.recoverySuggestion)
        #expect(unencodable.contains("Undo the edit"))
        #expect(unencodable.contains("export"))

        let ambiguous = try #require(CSVError.ambiguousOutput.recoverySuggestion)
        #expect(ambiguous.contains("Undo the edit"))
        #expect(ambiguous.contains("export"))
    }
}

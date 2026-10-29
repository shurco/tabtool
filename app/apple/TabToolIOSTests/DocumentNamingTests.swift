import Foundation
import Testing
@testable import TabTool

@Suite struct DocumentNamingTests {
    @Test func theFirstNewDocumentHasThePlainName() {
        #expect(DocumentNaming.newDocumentName(existing: []) == "Untitled.csv")
    }

    @Test func newDocumentsCountUp() {
        #expect(DocumentNaming.newDocumentName(existing: ["untitled.csv"]) == "Untitled 2.csv")
        #expect(DocumentNaming.newDocumentName(existing: ["untitled.csv", "untitled 2.csv"])
            == "Untitled 3.csv")
    }

    @Test func newDocumentNamesIgnoreCase() {
        #expect(DocumentNaming.newDocumentName(existing: ["UNTITLED.CSV"]) == "Untitled 2.csv")
    }

    @Test func anImportKeepsItsNameWhenFree() {
        #expect(DocumentNaming.availableName(for: "data.csv", existing: ["other.csv"]) == "data.csv")
    }

    @Test func anImportGetsANumberWhenTheNameIsTaken() {
        #expect(DocumentNaming.availableName(for: "data.csv", existing: ["data.csv"]) == "data 2.csv")
        #expect(DocumentNaming.availableName(for: "data.csv", existing: ["data.csv", "data 2.csv"])
            == "data 3.csv")
    }

    @Test func aNameWithoutExtensionKeepsItsShape() {
        #expect(DocumentNaming.availableName(for: "notes", existing: ["notes"]) == "notes 2")
    }
}

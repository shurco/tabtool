import Foundation
import Testing
@testable import TabTool

@Suite struct DroppedFilesTests {
    @Test func keepsTableFilesCaseInsensitively() {
        let urls = ["a.csv", "b.TSV", "c.txt", "d.tab", "e.dat"]
            .map { URL(fileURLWithPath: "/tmp/\($0)") }
        #expect(DroppedFiles.tableFiles(in: urls) == urls)
    }

    @Test func dropsOtherExtensionsAndRemoteURLs() {
        let urls = [URL(fileURLWithPath: "/tmp/a.png"),
                    URL(fileURLWithPath: "/tmp/noext"),
                    URL(string: "https://example.com/a.csv")!]
        #expect(DroppedFiles.tableFiles(in: urls).isEmpty)
    }
}

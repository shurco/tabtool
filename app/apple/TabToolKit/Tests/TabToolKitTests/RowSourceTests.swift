import Testing
@testable import TabToolKit

@Test func csvTableConformsToRowSource() {
    let table = CSVTable(rows: [["a", "b"], ["c"]], headers: ["h1", "h2"])
    let source: any RowSource = table
    #expect(source.rowCount == 2)
    #expect(source.columnCount == 2)
    #expect(source.columnIDs == table.columns.map(\.id))
    #expect(source.headers == ["h1", "h2"])
    #expect(source.cell(row: 1, column: 1) == "")
    #expect(source.cell(row: 9, column: 0) == "")
}

@Test func thresholdIs100MiB() {
    #expect(LargeFile.thresholdBytes == 104_857_600)
}

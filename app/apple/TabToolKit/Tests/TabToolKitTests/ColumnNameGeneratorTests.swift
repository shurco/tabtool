import Testing
@testable import TabToolKit

@Test(arguments: [
    (0, "A"), (25, "Z"), (26, "AA"), (27, "AB"),
    (51, "AZ"), (52, "BA"), (701, "ZZ"), (702, "AAA")
])
func columnNames(index: Int, expected: String) {
    #expect(ColumnNameGenerator.name(for: index) == expected)
}

@Test func negativeIndexHasNoName() {
    #expect(ColumnNameGenerator.name(for: -1) == "")
}

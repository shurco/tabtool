import Foundation

/// The rows and columns selected in the grid.
///
/// A table selection is either a set of rows or a set of columns; the grid
/// clears the other axis when the user changes it. Toolbar and menu commands
/// read this state to decide what they operate on.
struct GridSelection: Equatable, Sendable {
    var rows: IndexSet
    var columns: IndexSet

    init(rows: IndexSet = [], columns: IndexSet = []) {
        self.rows = rows
        self.columns = columns
    }
}

import Foundation

/// The rows and columns selected in the grid.
///
/// The grid fills this state in a later task; the document only publishes it.
struct GridSelection: Equatable {
    var rows: IndexSet = []
    var columns: IndexSet = []
}

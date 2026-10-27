/// Owns the grid's in-progress cell edit.
///
/// The grid coordinator is the only object that can end the AppKit field
/// editor. The document holds a weak reference so structural mutations, save
/// key equivalents, and window close can commit the edit against the current
/// row and column before the table changes.
@MainActor
protocol DocumentEditingSession: AnyObject {
    /// Commits and ends the active field editor, if one is open.
    func endEditing()
}

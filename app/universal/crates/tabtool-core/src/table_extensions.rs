//! The file extensions TabTool opens as tables.

use std::path::Path;

/// The extensions the app accepts and the open panel offers.
///
/// These match the Apple `Info.plist` document types and `DroppedFiles`.
/// `tabtool-platform/tests/kotlin_extensions.rs` keeps the Kotlin bridge's
/// copy in sync.
pub const TABLE_EXTENSIONS: [&str; 5] = ["csv", "tsv", "txt", "tab", "dat"];

/// Whether `extension` is a table extension, ignoring ASCII case.
pub fn is_table_extension(extension: &str) -> bool {
    TABLE_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension))
}

/// Whether `path` has a table extension.
pub fn has_table_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(is_table_extension)
}

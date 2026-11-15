//! Keeps the Kotlin bridge's accepted extensions in sync with the Rust list.

use tabtool_core::TABLE_EXTENSIONS;

const DOCUMENT_FILES: &str = include_str!(
    "../../../apps/tabtool-android/android/app/src/main/java/app/shurco/tabtool/DocumentFiles.kt"
);
const MARKER: &str = "val tableExtensions = setOf(";

/// The extension literals in the Kotlin `tableExtensions` set.
fn kotlin_extensions() -> Vec<String> {
    let start = DOCUMENT_FILES
        .find(MARKER)
        .expect("DocumentFiles.kt declares tableExtensions")
        + MARKER.len();
    let end = start
        + DOCUMENT_FILES[start..]
            .find(')')
            .expect("tableExtensions is a closed setOf call");

    DOCUMENT_FILES[start..end]
        .split(',')
        .map(|item| item.trim().trim_matches('"'))
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_kotlin_bridge_accepts_exactly_the_rust_extensions() {
    let mut kotlin = kotlin_extensions();
    kotlin.sort();
    let mut rust = TABLE_EXTENSIONS.map(str::to_owned).to_vec();
    rust.sort();

    assert_eq!(kotlin, rust);
}

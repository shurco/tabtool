//! Keeps the Kotlin bridge's accepted extensions in sync with the Rust list.

use tabtool_platform::TABLE_EXTENSIONS;

const BRIDGE: &str = include_str!(
    "../../../apps/tabtool-android/android/app/src/main/java/app/shurco/tabtool/TabToolBridge.kt"
);
const MARKER: &str = "private val tableExtensions = setOf(";

/// The extension literals in the Kotlin `tableExtensions` set.
fn kotlin_extensions() -> Vec<String> {
    let start = BRIDGE
        .find(MARKER)
        .expect("TabToolBridge.kt declares tableExtensions")
        + MARKER.len();
    let end = start
        + BRIDGE[start..]
            .find(')')
            .expect("tableExtensions is a closed setOf call");

    BRIDGE[start..end]
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

//! The apps show the release version from `app/universal/version.json`.

const VERSION_FILE: &str = include_str!("../../../version.json");

#[test]
fn the_version_is_the_release_name_in_version_json() {
    let file: serde_json::Value = serde_json::from_str(VERSION_FILE).expect("version.json parses");

    assert_eq!(Some(tabtool_core::VERSION), file["name"].as_str());
}

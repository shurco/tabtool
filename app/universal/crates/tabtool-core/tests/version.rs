//! The apps show the release version from the `universal` entry of
//! `app/version.json`.

const VERSION_FILE: &str = include_str!("../../../../version.json");

#[test]
fn the_version_is_the_universal_name_in_version_json() {
    let file: serde_json::Value = serde_json::from_str(VERSION_FILE).expect("version.json parses");

    assert_eq!(
        Some(tabtool_core::VERSION),
        file["universal"]["name"].as_str()
    );
}

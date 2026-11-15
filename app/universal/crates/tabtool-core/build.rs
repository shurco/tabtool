//! Exposes the release name from `app/universal/version.json` as
//! `TABTOOL_VERSION`, so every app shows the version that Gradle and the
//! release workflow use instead of the Cargo package version.

const VERSION_FILE: &str = "../../version.json";

fn main() {
    println!("cargo:rerun-if-changed={VERSION_FILE}");
    let text = std::fs::read_to_string(VERSION_FILE).expect("version.json is readable");
    let file: serde_json::Value = serde_json::from_str(&text).expect("version.json parses");
    let name = file["name"]
        .as_str()
        .expect("version.json has a string name");
    println!("cargo:rustc-env=TABTOOL_VERSION={name}");
}

//! `xtask version`: reads and bumps the `universal` entry of `app/version.json`.
//!
//! The entry holds the Rust apps' version, `{"build": "1", "name": "1.0.0"}`:
//! Gradle reads it for `versionName` and `versionCode`, and `tabtool-core`
//! exposes `name` as `tabtool_core::VERSION`. The `macos` and `ios` entries in
//! the same file belong to `scripts/lib/version` and are kept as they are.
//! Bumping follows `scripts/lib/version`: the build number always increases,
//! and patch, minor and major also move the semantic version.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// The accepted bump kinds, from smallest to largest.
const KINDS: [&str; 4] = ["build", "patch", "minor", "major"];

/// The entry this tool owns in the shared version file.
const ENTRY: &str = "universal";

/// A validated version name and build number.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    build: u64,
}

impl Version {
    /// The `x.y.z` name, which becomes the Android `versionName`.
    fn name(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// `<name> (<build>)`, the line every command prints.
    fn render(&self) -> String {
        format!("{} ({})", self.name(), self.build)
    }

    /// Raises the build number and, for every kind but `build`, the name.
    fn bump(&mut self, kind: &str) -> Result<(), String> {
        match kind {
            "build" => {}
            "patch" => self.patch += 1,
            "minor" => {
                self.minor += 1;
                self.patch = 0;
            }
            "major" => {
                self.major += 1;
                self.minor = 0;
                self.patch = 0;
            }
            _ => {
                return Err(format!(
                    "unknown bump kind {kind:?}: use {}",
                    KINDS.join(", ")
                ));
            }
        }
        self.build += 1;
        Ok(())
    }
}

/// Prints the current version.
pub fn print() -> Result<(), String> {
    let version = load(&version_path())?;
    println!("Version ({ENTRY}): {}", version.render());
    Ok(())
}

/// Bumps the version file and prints the new version.
pub fn run_bump(kind: &str) -> Result<(), String> {
    let path = version_path();
    let mut version = load(&path)?;
    version.bump(kind)?;
    store(&path, &version)?;
    println!("Version ({ENTRY}, {kind}): {}", version.render());
    Ok(())
}

fn version_path() -> PathBuf {
    crate::workspace_root().join("../version.json")
}

/// Reads the whole version file as a JSON object.
fn read(path: &Path) -> Result<Map<String, Value>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(data)) => Ok(data),
        Ok(_) => Err(format!("{} must hold a JSON object", path.display())),
        Err(error) => Err(format!("{} is not valid JSON: {error}", path.display())),
    }
}

/// Reads and validates the `universal` entry.
fn load(path: &Path) -> Result<Version, String> {
    let data = read(path)?;
    let entry = data
        .get(ENTRY)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{} has no {ENTRY:?} object", path.display()))?;
    parse(entry).map_err(|error| format!("{} {ENTRY}: {error}", path.display()))
}

/// Reads the two fields; both are strings, as in the Apple entries.
fn parse(entry: &Map<String, Value>) -> Result<Version, String> {
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no string \"name\"")?;
    let build = entry
        .get("build")
        .and_then(Value::as_str)
        .ok_or("no string \"build\"")?;
    let (major, minor, patch) = parse_name(name)?;
    let build = parse_number(build).map_err(|error| format!("invalid build: {error}"))?;
    if build == 0 {
        return Err("invalid build: must be at least 1".to_owned());
    }
    Ok(Version {
        major,
        minor,
        patch,
        build,
    })
}

/// Splits `x.y.z` into its three numbers; extra, empty or non-numeric
/// components are rejected.
fn parse_name(name: &str) -> Result<(u64, u64, u64), String> {
    let parts: Vec<&str> = name.split('.').collect();
    let numbers = match parts.as_slice() {
        [major, minor, patch] => [*major, *minor, *patch],
        _ => return Err(format!("invalid version name {name:?}: must be x.y.z")),
    };
    let mut values = [0_u64; 3];
    for (value, part) in values.iter_mut().zip(numbers) {
        *value = parse_number(part)
            .map_err(|error| format!("invalid version name {name:?}: {error}"))?;
    }
    Ok((values[0], values[1], values[2]))
}

/// Accepts only a non-empty run of ASCII digits.
fn parse_number(text: &str) -> Result<u64, String> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{text:?} is not a number"));
    }
    text.parse()
        .map_err(|error| format!("{text:?} is out of range: {error}"))
}

/// Rewrites the `universal` entry and keeps every other key. Keys come out
/// sorted, as `scripts/lib/version` writes them. The file is replaced through
/// a temporary, so an interrupted bump cannot corrupt it.
fn store(path: &Path, version: &Version) -> Result<(), String> {
    let mut data = read(path)?;
    data.insert(
        ENTRY.to_owned(),
        json!({"name": version.name(), "build": version.build.to_string()}),
    );
    let mut text = serde_json::to_string_pretty(&data)
        .map_err(|error| format!("cannot render the version: {error}"))?;
    text.push('\n');

    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh directory per test that removes itself, so the tests run in
    /// parallel and leave nothing behind.
    struct TemporaryDirectory {
        path: PathBuf,
    }

    impl TemporaryDirectory {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "tabtool-version-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("the temporary directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// The shared file: the Apple entries belong to `scripts/lib/version`.
    const SAMPLE: &str = r#"{
  "_comment": "Release versions",
  "ios": {"build": "7", "name": "2.0.0"},
  "macos": {"build": "3", "name": "1.0.1"},
  "universal": {"build": "1", "name": "1.0.0"}
}"#;

    fn write_sample(directory: &Path) -> PathBuf {
        let path = directory.join("version.json");
        fs::write(&path, SAMPLE).expect("the sample version file");
        path
    }

    #[test]
    fn a_build_bump_raises_only_the_build_number() {
        let directory = TemporaryDirectory::new();
        let mut version = load(&write_sample(directory.path())).expect("a valid version");
        version.bump("build").expect("a valid kind");
        assert_eq!(version.name(), "1.0.0");
        assert_eq!(version.build, 2);
    }

    #[test]
    fn patch_minor_and_major_bumps_move_the_name_and_the_build() {
        let directory = TemporaryDirectory::new();
        let mut version = load(&write_sample(directory.path())).expect("a valid version");
        version.bump("patch").expect("a valid kind");
        assert_eq!(version.render(), "1.0.1 (2)");
        version.bump("minor").expect("a valid kind");
        assert_eq!(version.render(), "1.1.0 (3)");
        version.bump("major").expect("a valid kind");
        assert_eq!(version.render(), "2.0.0 (4)");
    }

    #[test]
    fn an_unknown_bump_kind_is_rejected() {
        let directory = TemporaryDirectory::new();
        let mut version = load(&write_sample(directory.path())).expect("a valid version");
        assert!(version.bump("nope").is_err());
        assert_eq!(version.render(), "1.0.0 (1)");
    }

    #[test]
    fn malformed_names_and_builds_are_rejected() {
        let directory = TemporaryDirectory::new();
        for (name, build) in [
            ("1.0", "1"),
            ("1.2.3.4", "1"),
            ("1..3", "1"),
            ("1.2.x", "1"),
            (" 1.2.3", "1"),
            ("1.2.3", "0"),
            ("1.2.3", "-1"),
            ("1.2.3", "1.5"),
            ("1.2.3", ""),
        ] {
            let path = directory.path().join("version.json");
            fs::write(
                &path,
                format!("{{\"universal\": {{\"name\": \"{name}\", \"build\": \"{build}\"}}}}"),
            )
            .expect("the version file");
            assert!(
                load(&path).is_err(),
                "{name:?} with build {build:?} must be rejected"
            );
        }
    }

    #[test]
    fn a_file_without_the_universal_entry_is_rejected() {
        let directory = TemporaryDirectory::new();
        let path = directory.path().join("version.json");
        fs::write(&path, r#"{"macos": {"name": "1.0.0", "build": "1"}}"#)
            .expect("the version file");
        assert!(load(&path).is_err());
    }

    #[test]
    fn a_missing_file_cannot_be_read() {
        let directory = TemporaryDirectory::new();
        assert!(load(&directory.path().join("version.json")).is_err());
    }

    #[test]
    fn storing_rewrites_only_the_universal_entry_and_leaves_no_temporary() {
        let directory = TemporaryDirectory::new();
        let path = write_sample(directory.path());
        let mut version = load(&path).expect("a valid version");
        version.bump("patch").expect("a valid kind");
        store(&path, &version).expect("a written version");

        assert_eq!(load(&path).expect("the round trip"), version);
        assert_eq!(
            fs::read_to_string(&path).expect("the written file"),
            concat!(
                "{\n",
                "  \"_comment\": \"Release versions\",\n",
                "  \"ios\": {\n    \"build\": \"7\",\n    \"name\": \"2.0.0\"\n  },\n",
                "  \"macos\": {\n    \"build\": \"3\",\n    \"name\": \"1.0.1\"\n  },\n",
                "  \"universal\": {\n    \"build\": \"2\",\n    \"name\": \"1.0.1\"\n  }\n",
                "}\n"
            )
        );
        let names: Vec<String> = fs::read_dir(directory.path())
            .expect("the directory")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, ["version.json"]);
    }

    #[test]
    fn the_component_parser_accepts_plain_numbers_only() {
        assert_eq!(parse_name("10.20.30"), Ok((10, 20, 30)));
        assert!(parse_name("01.2.3").is_ok());
        assert!(parse_name("1.2.3\n").is_err());
    }
}

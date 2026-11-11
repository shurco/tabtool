//! Development-only fixtures and generators for TabTool tests.

mod generators;

pub use generators::{LargeFixture, LargeOptions, generate_large_csv, make_benchmark_csv};

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// Manifest paths are relative to the test data root, below this directory.
const MANIFEST_PREFIX: &str = "fixtures/";

const FIXTURE_NAMES: &[&str] = &[
    "Heuristic/heuristic-comma-separated-people-1.csv",
    "Heuristic/heuristic-comma-separated-people-2.csv",
    "Heuristic/heuristic-comma-separated-places.csv",
    "Heuristic/heuristic-config1.csv",
    "Heuristic/heuristic-config10.csv",
    "Heuristic/heuristic-config11.csv",
    "Heuristic/heuristic-config2.csv",
    "Heuristic/heuristic-config3.csv",
    "Heuristic/heuristic-config4.csv",
    "Heuristic/heuristic-config5.csv",
    "Heuristic/heuristic-config6.csv",
    "Heuristic/heuristic-config7.csv",
    "Heuristic/heuristic-config8.csv",
    "Heuristic/heuristic-config9.csv",
    "Heuristic/heuristic-first-row-longer.csv",
    "Heuristic/heuristic-first-row-one-row-shorter.csv",
    "Heuristic/heuristic-first-row-with-number.csv",
    "Heuristic/heuristic-gbk.csv",
    "Heuristic/heuristic-no-utf-encoding.csv",
    "Heuristic/macroman-tab-sample.csv",
    "Performance/test-doc-quoted.csv",
    "Performance/test-doc-unquoted.csv",
    "Reading/blank-lines.csv",
    "Reading/comma-separated-quote.csv",
    "Reading/comma-separated.csv",
    "Reading/invalid-encoding.csv",
    "Reading/missing-backslash-afterValue.csv",
    "Reading/missing-backslash-beforeValue.csv",
    "Reading/missing-quote-atEnd.csv",
    "Reading/missing-value-for-backslash-inquote.csv",
    "Reading/missing-value-for-backslash.csv",
    "Reading/quote-backslash-escape.csv",
    "Reading/quote-in-unquoted-value.csv",
    "Reading/quote-quote-escape.csv",
    "Reading/semicolon-separated.csv",
    "Reading/tooMany-quotes-atEnd.csv",
];

/// Root of the shared test data directory (`app/universal/testdata`).
pub fn testdata_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("testdata")
}

/// Directory holding the fixtures copied from the Apple test bundle.
pub fn fixtures_dir() -> PathBuf {
    testdata_dir().join("fixtures")
}

/// Path of the committed SHA-256 manifest.
pub fn manifest_path() -> PathBuf {
    testdata_dir().join("fixtures.sha256.json")
}

/// Reads a fixture by its path below `fixtures/`, such as `Reading/quoted.csv`.
///
/// # Panics
///
/// Panics when the fixture is missing or cannot be read.
pub fn fixture(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    std::fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read fixture {}: {error}", path.display()))
}

/// Every fixture copied from the Apple test bundle, relative to `fixtures/`.
pub fn fixture_names() -> &'static [&'static str] {
    FIXTURE_NAMES
}

/// Hashes recorded in `fixtures.sha256.json`, keyed by names accepted by
/// [`fixture`].
///
/// # Panics
///
/// Panics when the manifest is missing or malformed.
pub fn manifest() -> BTreeMap<String, String> {
    let path = manifest_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read manifest {}: {error}", path.display()));
    let document: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("cannot parse manifest {}: {error}", path.display()));
    let files = document
        .get("files")
        .and_then(serde_json::Value::as_object)
        .unwrap_or_else(|| panic!("manifest {} has no \"files\" object", path.display()));

    files
        .iter()
        .map(|(name, hash)| {
            let name = name
                .strip_prefix(MANIFEST_PREFIX)
                .unwrap_or_else(|| panic!("manifest path {name} is outside {MANIFEST_PREFIX}"));
            let hash = hash
                .as_str()
                .unwrap_or_else(|| panic!("manifest hash for {name} is not a string"));
            (name.to_owned(), hash.to_owned())
        })
        .collect()
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}

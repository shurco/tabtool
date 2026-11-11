//! Regenerates `testdata/fixtures.sha256.json` from the copied fixtures.
//!
//! Run with `cargo run -p tabtool-testdata --bin manifest`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use tabtool_testdata::{manifest_path, sha256_hex, testdata_dir};

fn walk(dir: &Path, root: &Path, files: &mut BTreeMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk(&path, root, files)?;
        } else if file_type.is_file() {
            let name = path
                .strip_prefix(root)
                .expect("walked path is below the test data root")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(name, sha256_hex(&fs::read(&path)?));
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let root = testdata_dir();
    let mut files = BTreeMap::new();
    walk(&root.join("fixtures"), &root, &mut files)?;

    let document = serde_json::json!({ "files": files });
    let mut text = serde_json::to_string_pretty(&document).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(manifest_path(), text)
}

//! Settings JSON values for paths, keeping non-UTF-8 paths lossless.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Stores a UTF-8 path as a string; a non-UTF-8 path keeps its exact bytes
/// (Unix) or UTF-16 units (Windows) in an object so nothing is lost.
pub(super) fn path_value(path: &Path) -> Value {
    match path.to_str() {
        Some(text) => Value::String(text.to_string()),
        None => non_utf8_path_value(path),
    }
}

pub(super) fn path_from_value(value: &Value) -> Option<PathBuf> {
    if let Some(text) = value.as_str() {
        return Some(PathBuf::from(text));
    }
    non_utf8_path_from(value.as_object()?)
}

#[cfg(unix)]
fn non_utf8_path_value(path: &Path) -> Value {
    use std::os::unix::ffi::OsStrExt;

    let bytes = path
        .as_os_str()
        .as_bytes()
        .iter()
        .map(|byte| Value::from(*byte))
        .collect();
    let mut map = Map::new();
    map.insert("unixBytes".to_string(), Value::Array(bytes));
    Value::Object(map)
}

#[cfg(unix)]
fn non_utf8_path_from(object: &Map<String, Value>) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let bytes: Vec<u8> = object
        .get("unixBytes")?
        .as_array()?
        .iter()
        .map(|value| value.as_u64().and_then(|value| u8::try_from(value).ok()))
        .collect::<Option<_>>()?;
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

#[cfg(windows)]
fn non_utf8_path_value(path: &Path) -> Value {
    use std::os::windows::ffi::OsStrExt;

    let units = path.as_os_str().encode_wide().map(Value::from).collect();
    let mut map = Map::new();
    map.insert("windowsWide".to_string(), Value::Array(units));
    Value::Object(map)
}

#[cfg(windows)]
fn non_utf8_path_from(object: &Map<String, Value>) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let units: Vec<u16> = object
        .get("windowsWide")?
        .as_array()?
        .iter()
        .map(|value| value.as_u64().and_then(|value| u16::try_from(value).ok()))
        .collect::<Option<_>>()?;
    Some(PathBuf::from(OsString::from_wide(&units)))
}

#[cfg(not(any(unix, windows)))]
fn non_utf8_path_value(path: &Path) -> Value {
    Value::String(path.to_string_lossy().into_owned())
}

#[cfg(not(any(unix, windows)))]
fn non_utf8_path_from(_object: &Map<String, Value>) -> Option<PathBuf> {
    None
}

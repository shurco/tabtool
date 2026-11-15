//! Reading and atomically writing the settings JSON.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use tabtool_i18n::Language;
use tabtool_settings::{
    LARGE_FILE_THRESHOLDS_MIB, QuoteStyle, RECENT_DOCUMENTS_LIMIT, Settings, keys, valid_row_height,
};

use super::encoding::{encoding_from_foundation_id, foundation_encoding_id};

/// Reads the settings at `path`, falling back per value to the defaults.
pub(crate) fn load(path: &Path) -> Settings {
    let map = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    from_map(&map)
}

/// Writes the settings atomically: a temporary file in the same directory is
/// renamed over the target.
pub(crate) fn save(settings: &Settings, path: &Path) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(&to_json(settings))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    let mut temp_path = temp.into_temp_path();

    if path.exists() {
        match tabtool_large::replace_file(path, &temp_path) {
            Ok(()) => {
                temp_path.disable_cleanup(true);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                temp_path.persist(path).map_err(|error| error.error)
            }
            Err(error) => Err(error),
        }
    } else {
        temp_path.persist(path).map_err(|error| error.error)
    }
}

fn from_map(map: &Map<String, Value>) -> Settings {
    let defaults = Settings::default();
    Settings {
        show_format_bar: bool_value(map, keys::SHOW_FORMAT_BAR, defaults.show_format_bar),
        app_language: string_value(map, keys::APP_LANGUAGE)
            .and_then(|value| Language::from_persisted(&value))
            .unwrap_or(defaults.app_language),
        new_document_encoding: map
            .get(keys::NEW_DOCUMENT_ENCODING)
            .and_then(Value::as_i64)
            .and_then(encoding_from_foundation_id)
            .unwrap_or(defaults.new_document_encoding),
        new_document_separator: string_value(map, keys::NEW_DOCUMENT_SEPARATOR)
            .filter(|value| matches!(value.as_str(), "," | ";" | "\t" | "|"))
            .unwrap_or(defaults.new_document_separator),
        new_document_quote_style: string_value(map, keys::NEW_DOCUMENT_QUOTE_STYLE)
            .and_then(|value| QuoteStyle::from_persisted(&value))
            .unwrap_or(defaults.new_document_quote_style),
        new_document_decimal_mark: string_value(map, keys::NEW_DOCUMENT_DECIMAL_MARK)
            .filter(|value| matches!(value.as_str(), "." | ","))
            .unwrap_or(defaults.new_document_decimal_mark),
        new_document_first_row_as_header: bool_value(
            map,
            keys::NEW_DOCUMENT_FIRST_ROW_AS_HEADER,
            defaults.new_document_first_row_as_header,
        ),
        grid_alternating_rows: bool_value(
            map,
            keys::GRID_ALTERNATING_ROWS,
            defaults.grid_alternating_rows,
        ),
        grid_row_height: map
            .get(keys::GRID_ROW_HEIGHT)
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .map(valid_row_height)
            .unwrap_or(defaults.grid_row_height),
        grid_shows_row_numbers: bool_value(
            map,
            keys::GRID_SHOWS_ROW_NUMBERS,
            defaults.grid_shows_row_numbers,
        ),
        large_file_threshold_mib: map
            .get(keys::LARGE_FILE_THRESHOLD_MIB)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| LARGE_FILE_THRESHOLDS_MIB.contains(value))
            .unwrap_or(defaults.large_file_threshold_mib),
        recent_documents: recent_documents(map),
        review_first_launch: string_value(map, keys::REVIEW_FIRST_LAUNCH),
        review_active_days: u32_value(map, keys::REVIEW_ACTIVE_DAYS),
        review_last_active_day: string_value(map, keys::REVIEW_LAST_ACTIVE_DAY),
        review_engagements: u32_value(map, keys::REVIEW_ENGAGEMENTS),
        review_last_prompted_version: string_value(map, keys::REVIEW_LAST_PROMPTED_VERSION),
        review_last_prompted_at: string_value(map, keys::REVIEW_LAST_PROMPTED_AT),
    }
}

fn to_json(settings: &Settings) -> Value {
    let recent: Vec<Value> = settings
        .recent_documents
        .iter()
        .take(RECENT_DOCUMENTS_LIMIT)
        .map(|path| path_value(path))
        .collect();
    let mut map = Map::new();
    map.insert(
        keys::SHOW_FORMAT_BAR.to_string(),
        settings.show_format_bar.into(),
    );
    map.insert(
        keys::APP_LANGUAGE.to_string(),
        settings.app_language.persisted().into(),
    );
    map.insert(
        keys::NEW_DOCUMENT_ENCODING.to_string(),
        foundation_encoding_id(settings.new_document_encoding).into(),
    );
    map.insert(
        keys::NEW_DOCUMENT_SEPARATOR.to_string(),
        settings.new_document_separator.as_str().into(),
    );
    map.insert(
        keys::NEW_DOCUMENT_QUOTE_STYLE.to_string(),
        settings.new_document_quote_style.persisted().into(),
    );
    map.insert(
        keys::NEW_DOCUMENT_DECIMAL_MARK.to_string(),
        settings.new_document_decimal_mark.as_str().into(),
    );
    map.insert(
        keys::NEW_DOCUMENT_FIRST_ROW_AS_HEADER.to_string(),
        settings.new_document_first_row_as_header.into(),
    );
    map.insert(
        keys::GRID_ALTERNATING_ROWS.to_string(),
        settings.grid_alternating_rows.into(),
    );
    map.insert(
        keys::GRID_ROW_HEIGHT.to_string(),
        settings.grid_row_height.into(),
    );
    map.insert(
        keys::GRID_SHOWS_ROW_NUMBERS.to_string(),
        settings.grid_shows_row_numbers.into(),
    );
    map.insert(
        keys::LARGE_FILE_THRESHOLD_MIB.to_string(),
        settings.large_file_threshold_mib.into(),
    );
    map.insert(keys::RECENT_DOCUMENTS.to_string(), Value::Array(recent));
    for (key, value) in [
        (keys::REVIEW_FIRST_LAUNCH, &settings.review_first_launch),
        (
            keys::REVIEW_LAST_ACTIVE_DAY,
            &settings.review_last_active_day,
        ),
        (
            keys::REVIEW_LAST_PROMPTED_VERSION,
            &settings.review_last_prompted_version,
        ),
        (
            keys::REVIEW_LAST_PROMPTED_AT,
            &settings.review_last_prompted_at,
        ),
    ] {
        let value = value.as_deref().map_or(Value::Null, Value::from);
        map.insert(key.to_string(), value);
    }
    map.insert(
        keys::REVIEW_ACTIVE_DAYS.to_string(),
        settings.review_active_days.into(),
    );
    map.insert(
        keys::REVIEW_ENGAGEMENTS.to_string(),
        settings.review_engagements.into(),
    );
    Value::Object(map)
}

fn bool_value(map: &Map<String, Value>, key: &str, default: bool) -> bool {
    map.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn string_value(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(Value::as_str).map(str::to_string)
}

fn u32_value(map: &Map<String, Value>, key: &str) -> u32 {
    map.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}

fn recent_documents(map: &Map<String, Value>) -> Vec<PathBuf> {
    let Some(items) = map.get(keys::RECENT_DOCUMENTS).and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for item in items.iter().take(RECENT_DOCUMENTS_LIMIT) {
        let Some(path) = path_from_value(item) else {
            return Vec::new();
        };
        paths.push(path);
    }
    paths
}

/// Stores a UTF-8 path as a string; a non-UTF-8 path keeps its exact bytes
/// (Unix) or UTF-16 units (Windows) in an object so nothing is lost.
fn path_value(path: &Path) -> Value {
    match path.to_str() {
        Some(text) => Value::String(text.to_string()),
        None => non_utf8_path_value(path),
    }
}

fn path_from_value(value: &Value) -> Option<PathBuf> {
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

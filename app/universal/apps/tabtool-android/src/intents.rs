//! Documents other apps send to TabTool.
//!
//! `ACTION_VIEW` and `ACTION_SEND` arrive as content URIs. Kotlin copies the
//! stream into a hidden staging file inside the private documents directory
//! and reports the staging name with the display name. This module validates
//! the name, picks a collision-free final name and moves the copy into place;
//! the app then opens it. A failure never leaves the staging file behind.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tabtool_ui::mobile::available_name;

/// The extensions the app accepts, matching the manifest and the picker.
const TABLE_EXTENSIONS: [&str; 5] = ["csv", "tsv", "txt", "tab", "dat"];

/// The longest file name the store keeps, before the extension.
const MAX_STEM: usize = 100;

/// The prefix of an intent staging file.
const STAGING_PREFIX: &str = ".incoming-";

/// How old an abandoned intent copy must be before startup removes it.
pub const STALE_STAGING_AGE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// A staged intent copy reported by the platform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedIntent {
    /// The hidden file name inside the documents directory.
    pub file_name: String,
    /// The display name the sending app reported.
    pub display_name: String,
}

/// What the platform reports for a document another app sent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntentEvent {
    /// The copy is staged and ready to adopt.
    Staged(StagedIntent),
    /// The copy failed; the string is the diagnostic detail.
    Failed(String),
}

/// Whether `path` has a table extension.
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            TABLE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// Makes `display_name` a safe single-segment file name.
///
/// Control characters and path separators are removed, a leading dot is
/// stripped so the copy cannot hide itself, and the stem is bounded. The
/// extension is kept only when the app accepts it; an unsupported or missing
/// extension takes `fallback_extension`.
pub fn safe_name(display_name: &str, fallback_extension: &str) -> String {
    let file_name = Path::new(display_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(display_name);
    let cleaned: String = file_name
        .chars()
        .filter(|character| !character.is_control() && *character != '/' && *character != '\\')
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.');

    let proposed = Path::new(cleaned);
    let extension = proposed
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            TABLE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        });
    let stem = proposed
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty() && *stem != "." && *stem != "..")
        .unwrap_or("Imported");
    let stem: String = stem.chars().take(MAX_STEM).collect();
    match extension {
        Some(extension) => format!("{stem}.{}", extension.to_ascii_lowercase()),
        None => format!("{stem}.{fallback_extension}"),
    }
}

/// Moves a staged copy to a collision-free name in `documents_dir`.
///
/// The staged file is removed on every failure, so an unsupported or lost
/// intent copy never accumulates in the private store.
pub fn adopt(documents_dir: &Path, staged: &StagedIntent) -> io::Result<PathBuf> {
    let staged_path = documents_dir.join(&staged.file_name);
    let result = adopt_inner(documents_dir, &staged_path, &staged.display_name);
    if result.is_err() {
        let _ = fs::remove_file(&staged_path);
    }
    result
}

fn adopt_inner(
    documents_dir: &Path,
    staged_path: &Path,
    display_name: &str,
) -> io::Result<PathBuf> {
    if staged_path.parent() != Some(documents_dir) || !staged_path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "the staged document is missing at {}",
                staged_path.display()
            ),
        ));
    }
    let fallback_extension = staged_path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            TABLE_EXTENSIONS
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "the staged document type is not supported: {}",
                    staged_path.display()
                ),
            )
        })?;
    let name = safe_name(display_name, fallback_extension);
    let existing = existing_names(documents_dir);
    let destination = documents_dir.join(available_name(&name, existing));
    fs::rename(staged_path, &destination)?;
    Ok(destination)
}

/// The file names in `directory`, including files the library hides.
pub fn existing_names(directory: &Path) -> Vec<String> {
    fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// Removes abandoned intent staging files older than `older_than`.
///
/// A freshly written staging file may belong to a copy that is still running,
/// so only old files are swept. Returns how many were removed.
pub fn clean_stale(directory: &Path, older_than: std::time::Duration) -> usize {
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    for entry in fs::read_dir(directory).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(STAGING_PREFIX) {
            continue;
        }
        let stale = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= older_than);
        if stale && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Whether `path` is directly inside `directory`.
pub fn is_within(directory: &Path, path: &Path) -> bool {
    path.parent() == Some(directory)
}

#[cfg(test)]
mod tests {
    use super::{StagedIntent, adopt, safe_name};
    use std::fs;

    #[test]
    fn names_lose_paths_controls_and_hidden_prefixes() {
        assert_eq!(safe_name("../data.csv", "txt"), "data.csv");
        assert_eq!(safe_name(".hidden.csv", "txt"), "hidden.csv");
        assert_eq!(safe_name("a\nb.csv", "txt"), "ab.csv");
        assert_eq!(safe_name("", "csv"), "Imported.csv");
        assert_eq!(safe_name("notes.md", "txt"), "notes.txt");
    }

    #[test]
    fn an_adopted_copy_gets_a_number_instead_of_replacing_a_document() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(directory.path().join("data.csv"), b"old").expect("the existing file");
        fs::write(directory.path().join(".incoming-1.csv"), b"new").expect("the staged copy");

        let path = adopt(
            directory.path(),
            &StagedIntent {
                file_name: ".incoming-1.csv".to_owned(),
                display_name: "data.csv".to_owned(),
            },
        )
        .expect("the copy is adopted");

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("data 2.csv")
        );
        assert_eq!(fs::read(&path).expect("the adopted bytes"), b"new");
        assert_eq!(
            fs::read(directory.path().join("data.csv")).expect("the original bytes"),
            b"old"
        );
        assert!(!directory.path().join(".incoming-1.csv").exists());
    }

    #[test]
    fn a_failed_adoption_removes_the_staging_file() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(directory.path().join(".incoming-2.bin"), b"x").expect("the staged copy");

        let error = adopt(
            directory.path(),
            &StagedIntent {
                file_name: ".incoming-2.bin".to_owned(),
                display_name: "x.bin".to_owned(),
            },
        );

        assert!(error.is_err());
        assert!(!directory.path().join(".incoming-2.bin").exists());
    }

    #[test]
    fn startup_removes_only_old_staging_files() {
        use super::{STALE_STAGING_AGE, clean_stale};
        use std::time::{Duration, UNIX_EPOCH};

        let directory = tempfile::tempdir().expect("temporary directory");
        let old = directory.path().join(".incoming-1.csv");
        let fresh = directory.path().join(".incoming-2.csv");
        let document = directory.path().join("keep.csv");
        fs::write(&old, b"old").expect("the old staging file");
        fs::write(&fresh, b"fresh").expect("the fresh staging file");
        fs::write(&document, b"doc").expect("the document");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&old)
            .expect("the old file is writable");
        file.set_times(std::fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1)))
            .expect("the old time is set");

        assert_eq!(clean_stale(directory.path(), STALE_STAGING_AGE), 1);

        assert!(!old.exists());
        assert!(fresh.exists());
        assert!(document.exists());
    }
}

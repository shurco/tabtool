//! Document names: new-document names, collision numbering and renames.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tabtool_core::has_table_extension;
use tabtool_i18n::{ERROR_GENERIC, LocalizedError};

use super::LibraryView;

/// The base name of a new document.
const NEW_DOCUMENT_BASE: &str = "Untitled";

impl LibraryView {
    /// Renames `path` to `proposed`, keeping the extension when it is omitted.
    ///
    /// A taken name gets " 2", " 3" before the extension instead of replacing
    /// another document.
    pub fn rename_document(
        &mut self,
        path: &Path,
        proposed: &str,
    ) -> Result<PathBuf, LocalizedError> {
        let trimmed = proposed.trim();
        if trimmed.is_empty() {
            return Ok(path.to_path_buf());
        }
        if trimmed == "."
            || trimmed == ".."
            || trimmed.starts_with('.')
            || trimmed.ends_with('.')
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains(':')
        {
            return Err(LocalizedError::new(
                ERROR_GENERIC,
                "the document name would hide the file or leave the folder",
            ));
        }
        let proposed_path = Path::new(trimmed);
        if proposed_path.extension().is_some() && !has_table_extension(proposed_path) {
            return Err(LocalizedError::new(
                ERROR_GENERIC,
                "the document name must end in a table extension",
            ));
        }
        let current = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let extension = path.extension().and_then(|extension| extension.to_str());
        let name = if trimmed.contains('.') {
            trimmed.to_owned()
        } else {
            match extension {
                Some(extension) => format!("{trimmed}.{extension}"),
                None => trimmed.to_owned(),
            }
        };
        if name == current {
            return Ok(path.to_path_buf());
        }
        let existing = self.files.existing_names();
        if name.eq_ignore_ascii_case(current) {
            // A case-only rename keeps the same file; numbering it would
            // leave two names for one document.
            if !existing.iter().any(|existing| existing == &name) {
                let destination = path.with_file_name(&name);
                if let Err(error) = self.files.rename(path, &destination) {
                    return Err(LocalizedError::new(ERROR_GENERIC, error.to_string()));
                }
                self.refresh();
                return Ok(destination);
            }
        }
        let available = available_name(&name, existing);
        let destination = path.with_file_name(available);
        if let Err(error) = self.files.rename(path, &destination) {
            return Err(LocalizedError::new(ERROR_GENERIC, error.to_string()));
        }
        self.refresh();
        Ok(destination)
    }
}

/// The new-document name: `Untitled.csv`, then `Untitled 2.csv`, and so on.
///
/// Names are compared case-insensitively because the app's folder lives on a
/// case-insensitive volume.
pub fn new_document_name(
    existing: impl IntoIterator<Item = impl AsRef<str>>,
    extension: &str,
) -> String {
    let taken = lowercased(existing);
    let base = format!("{NEW_DOCUMENT_BASE}.{extension}");
    if !taken.contains(&base.to_lowercase()) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{NEW_DOCUMENT_BASE} {index}.{extension}");
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        index += 1;
    }
}

/// The proposed name, or the same name with " 2", " 3" before the extension.
pub fn available_name(
    proposed: &str,
    existing: impl IntoIterator<Item = impl AsRef<str>>,
) -> String {
    let taken = lowercased(existing);
    if !taken.contains(&proposed.to_lowercase()) {
        return proposed.to_owned();
    }
    let path = Path::new(proposed);
    let extension = path.extension().and_then(|extension| extension.to_str());
    let base = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(proposed);
    let mut index = 2;
    loop {
        let candidate = match extension {
            Some(extension) => format!("{base} {index}.{extension}"),
            None => format!("{base} {index}"),
        };
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        index += 1;
    }
}

fn lowercased(names: impl IntoIterator<Item = impl AsRef<str>>) -> BTreeSet<String> {
    names
        .into_iter()
        .map(|name| name.as_ref().to_lowercase())
        .collect()
}

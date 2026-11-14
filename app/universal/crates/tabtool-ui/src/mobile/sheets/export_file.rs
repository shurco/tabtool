//! The temporary file a converted export hands to the platform picker.

use core::sync::atomic::{AtomicU64, Ordering};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The unique name of the next temporary export directory.
static NEXT_EXPORT: AtomicU64 = AtomicU64::new(1);

/// Writes `data` to a unique temporary directory the picker can copy.
pub(super) fn temporary_export(base: &Path, data: &[u8], filename: &str) -> io::Result<PathBuf> {
    let unique = NEXT_EXPORT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let directory = base.join(format!(".export-{}-{nanos}-{unique}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(filename);
    std::fs::write(&path, data)?;
    Ok(path)
}

/// Removes the temporary directory of an export after the copy.
pub(super) fn remove_temporary_export(path: &Path) {
    if let Some(directory) = path.parent() {
        let _ = std::fs::remove_dir_all(directory);
    }
}

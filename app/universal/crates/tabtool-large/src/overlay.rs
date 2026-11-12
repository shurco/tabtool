//! The edited cells of a large file, kept apart from its bytes.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

/// A cell address in the file: the physical row (header row included) and
/// column.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CellKey {
    /// Physical row, counting the header row when one is enabled.
    pub row: usize,
    /// Column, from 0.
    pub column: usize,
}

/// The edited cells of a large file, kept apart from the file's bytes.
///
/// A reference type so a keystroke changes one entry instead of copying a
/// map; [`CellOverlay::copy`] takes a snapshot when one is needed (for saving).
/// Every method locks briefly; no caller holds the lock while decoding.
#[derive(Debug, Default)]
pub struct CellOverlay {
    cells: Mutex<HashMap<CellKey, String>>,
}

impl CellOverlay {
    /// Creates an overlay with no edits.
    pub fn new() -> Self {
        Self::default()
    }

    /// The text of an edited cell, or `None` when the file bytes still apply.
    pub fn get(&self, key: CellKey) -> Option<String> {
        lock(&self.cells).get(&key).cloned()
    }

    /// Whether no cell is edited.
    pub fn is_empty(&self) -> bool {
        lock(&self.cells).is_empty()
    }

    /// The number of edited cells.
    pub fn count(&self) -> usize {
        lock(&self.cells).len()
    }

    /// A snapshot of the edits, for a save.
    pub fn copy(&self) -> HashMap<CellKey, String> {
        lock(&self.cells).clone()
    }

    /// Sets the text of a cell.
    pub fn set(&self, text: String, key: CellKey) {
        lock(&self.cells).insert(key, text);
    }

    /// Removes the edit of a cell, if any.
    pub fn remove(&self, key: CellKey) {
        lock(&self.cells).remove(&key);
    }

    /// Removes every edit.
    pub fn remove_all(&self) {
        lock(&self.cells).clear();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

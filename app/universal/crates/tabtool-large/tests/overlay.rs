//! Ported from `CellOverlayTests.swift`: the edited cells of a large file.

use std::collections::HashMap;

use tabtool_large::{CellKey, CellOverlay};

#[test]
fn set_read_remove() {
    let overlay = CellOverlay::new();
    let key = CellKey { row: 3, column: 1 };
    assert!(overlay.is_empty());
    assert_eq!(overlay.get(key), None);
    overlay.set("x".to_string(), key);
    assert_eq!(overlay.get(key), Some("x".to_string()));
    assert_eq!(overlay.count(), 1);
    assert_eq!(overlay.copy(), HashMap::from([(key, "x".to_string())]));
    overlay.remove(key);
    assert!(overlay.is_empty());
}

#[test]
fn remove_all_clears() {
    let overlay = CellOverlay::new();
    overlay.set("a".to_string(), CellKey { row: 0, column: 0 });
    overlay.set("b".to_string(), CellKey { row: 1, column: 0 });
    overlay.remove_all();
    assert!(overlay.is_empty());
}

#[test]
fn copy_is_a_snapshot() {
    let overlay = CellOverlay::new();
    let key = CellKey { row: 0, column: 0 };
    overlay.set("a".to_string(), key);
    let snapshot = overlay.copy();
    overlay.set("b".to_string(), key);
    assert_eq!(snapshot.get(&key), Some(&"a".to_string()));
}

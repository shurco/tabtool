//! Index-permutation helpers shared by row and column reordering.

use crate::RowSet;

/// Returns the order that moves every index in `moving` by `by` positions.
///
/// Result position `i` holds the old index to place there. Selected indexes
/// keep their relative gaps; the rest fill the free slots in ascending order.
/// Returns `None` for an empty selection, or when any selected index, or its
/// target, falls outside `0..count`.
pub fn shifted(count: usize, moving: &RowSet, by: isize) -> Option<Vec<usize>> {
    if moving.is_empty() {
        return None;
    }

    let count_signed = isize::try_from(count).ok()?;
    let mut slots: Vec<Option<usize>> = vec![None; count];

    for &index in moving {
        let index_signed = isize::try_from(index).ok()?;
        let target = index_signed.checked_add(by)?;
        if !(0..count_signed).contains(&index_signed) || !(0..count_signed).contains(&target) {
            return None;
        }
        slots[usize::try_from(target).ok()?] = Some(index);
    }

    let mut rest = (0..count).filter(|index| !moving.contains(index));
    for slot in &mut slots {
        if slot.is_none() {
            *slot = rest.next();
        }
    }

    slots.into_iter().collect()
}

/// Whether `order` contains each index in `0..count` exactly once.
pub fn is_permutation(order: &[usize], count: usize) -> bool {
    if order.len() != count {
        return false;
    }

    let mut seen = vec![false; count];
    for &index in order {
        if index >= count || std::mem::replace(&mut seen[index], true) {
            return false;
        }
    }
    true
}

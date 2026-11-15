//! Field names for structured exports (JSON keys, XML element names).

use std::collections::HashSet;

use crate::{CsvTable, column_name};

/// The document headers, or spreadsheet-style names (A, B, ...) for a table
/// without headers. An empty header takes its column's name. Names are not
/// deduplicated; see [`unique`].
pub(crate) fn names(table: &CsvTable) -> Vec<String> {
    table
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| match column.header.as_deref() {
            Some(header) if !header.is_empty() => header.to_owned(),
            _ => column_name(index),
        })
        .collect()
}

/// Makes repeated names distinct by appending `_2`, `_3`, ... while skipping
/// any suffixed name that already appears in `names`.
pub(crate) fn unique(names: Vec<String>) -> Vec<String> {
    let mut taken: HashSet<String> = names.iter().cloned().collect();
    let mut seen = HashSet::new();
    names
        .into_iter()
        .map(|name| {
            if seen.insert(name.clone()) {
                return name;
            }
            let renamed = (2..)
                .map(|counter| format!("{name}_{counter}"))
                .find(|candidate| !taken.contains(candidate))
                .expect("an unused suffix exists");
            taken.insert(renamed.clone());
            renamed
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::unique;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn duplicates_get_a_numeric_suffix() {
        assert_eq!(
            unique(strings(&["a", "a", "a"])),
            strings(&["a", "a_2", "a_3"])
        );
    }

    #[test]
    fn the_suffix_skips_names_already_taken() {
        assert_eq!(
            unique(strings(&["a", "a", "a_2"])),
            strings(&["a", "a_3", "a_2"])
        );
    }
}

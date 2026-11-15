//! Command-line parsing and the free-space guard.

use std::path::Path;

use tabtool_large::available_space;

use crate::measure::gib;

const DEFAULT_GIB: f64 = 5.0;
const BYTES_PER_GIB: u64 = 1 << 30;

pub fn requested_gib(mut arguments: impl Iterator<Item = String>) -> Result<f64, String> {
    let value = match arguments.next() {
        Some(text) => parse_gib(&text)?,
        None => DEFAULT_GIB,
    };
    if arguments.next().is_some() {
        return Err(usage());
    }
    Ok(value)
}

fn parse_gib(text: &str) -> Result<f64, String> {
    let value = text
        .parse::<f64>()
        .map_err(|_| format!("invalid size {text:?}; {}", usage()))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("invalid size {text:?}; {}", usage()));
    }
    Ok(value)
}

pub fn target_bytes(gigabytes: f64) -> u64 {
    (gigabytes * BYTES_PER_GIB as f64).round().max(1.0) as u64
}

/// Refuses to generate a file unless the temporary volume has three times its
/// size free.
pub fn ensure_free_space(directory: &Path, target: u64) -> Result<(), String> {
    let required = target.saturating_mul(3);
    let free = available_space(directory)
        .map_err(|error| format!("cannot read free disk space: {error}"))?;
    if free < required {
        return Err(format!(
            "not enough free disk space: {} needs three times its size free, {} available",
            gib(target),
            gib(free)
        ));
    }
    Ok(())
}

fn usage() -> String {
    "usage: bench-large [GiB]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{parse_gib, requested_gib, target_bytes};

    #[test]
    fn accepts_the_default_and_fractions() {
        assert_eq!(
            requested_gib(std::iter::empty::<String>()).expect("the default is valid"),
            5.0
        );
        assert_eq!(parse_gib("0.5").expect("a fraction is valid"), 0.5);
        assert_eq!(target_bytes(1.0), 1 << 30);
        assert_eq!(target_bytes(0.5), 1 << 29);
    }

    #[test]
    fn rejects_bad_sizes() {
        for size in ["", "0", "-1", "nan", "inf", "many"] {
            assert!(parse_gib(size).is_err(), "{size:?} is rejected");
        }
        assert!(
            requested_gib(["1", "2"].into_iter().map(str::to_owned)).is_err(),
            "extra arguments are rejected"
        );
    }
}

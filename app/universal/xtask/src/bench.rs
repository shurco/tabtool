//! `xtask bench-large`: runs the large-file benchmark in a release build.

use std::process::Command;

use crate::{run, workspace_root};

/// Runs `bench-large [GiB]`; a missing size uses the binary's 5 GiB default.
pub fn bench_large(size: Option<&str>) -> Result<(), String> {
    validate_size(size)?;
    let mut command = Command::new("cargo");
    command.current_dir(workspace_root()).args([
        "run",
        "--release",
        "-p",
        "tabtool-large",
        "--features",
        "bench",
        "--bin",
        "bench-large",
    ]);
    if let Some(size) = size {
        command.arg("--").arg(size);
    }
    run(&mut command)
}

fn validate_size(size: Option<&str>) -> Result<(), String> {
    let Some(size) = size else {
        return Ok(());
    };
    let invalid = || format!("invalid size {size:?}; usage: xtask bench-large [GiB]");
    let value = size.parse::<f64>().map_err(|_| invalid())?;
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_size;

    #[test]
    fn accepts_the_default_and_positive_sizes() {
        assert!(validate_size(None).is_ok());
        assert!(validate_size(Some("5")).is_ok());
        assert!(validate_size(Some("0.5")).is_ok());
    }

    #[test]
    fn rejects_bad_sizes() {
        for size in ["", "0", "-1", "nan", "inf", "many"] {
            assert!(validate_size(Some(size)).is_err(), "{size:?} is rejected");
        }
    }
}

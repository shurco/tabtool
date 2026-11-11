//! Repository maintenance tasks.
//!
//! `xtask golden` regenerates the Swift golden files; `xtask golden --check`
//! runs the Rust parity test without needing a Swift toolchain.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Locales that have a golden file.
const LOCALES: [&str; 6] = ["en_US", "de_DE", "fr_FR", "zh_CN", "ja_JP", "ko_KR"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("workspace root resolves")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root resolves")
}

fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot start {:?}: {error}", command.get_program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{:?} exited with {status}", command.get_program()))
    }
}

/// Regenerates every golden file by running the Swift tool once per locale.
fn regenerate() -> Result<(), String> {
    let repository = repo_root();
    let tool = repository.join("tools/swift-golden");
    let fixtures = repository.join("app/universal/testdata/fixtures");
    let golden = repository.join("app/universal/testdata/golden");

    for locale in LOCALES {
        let language = locale.split('_').next().expect("locale has a language");
        let output = golden.join(format!("{locale}.json"));
        println!("generating {}", output.display());
        run(Command::new("swift")
            .current_dir(&repository)
            .arg("run")
            .arg("--package-path")
            .arg(&tool)
            .arg("swift-golden")
            .arg(&fixtures)
            .arg(&output)
            .arg("-AppleLocale")
            .arg(locale)
            .arg("-AppleLanguages")
            .arg(format!("({language})")))?;
    }
    Ok(())
}

/// Runs the Rust parity test against the committed golden files.
fn check() -> Result<(), String> {
    run(Command::new("cargo").current_dir(workspace_root()).args([
        "test",
        "-p",
        "tabtool-core",
        "--test",
        "golden",
    ]))
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(command) = arguments.next() else {
        eprintln!("usage: xtask golden [--check]");
        return ExitCode::from(2);
    };

    let result = match (command.as_str(), arguments.next().as_deref()) {
        ("golden", None) => regenerate(),
        ("golden", Some("--check")) => check(),
        _ => {
            eprintln!("usage: xtask golden [--check]");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

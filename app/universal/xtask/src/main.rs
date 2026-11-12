//! Repository maintenance tasks.
//!
//! `xtask golden` regenerates the Swift golden files; `xtask golden --check`
//! runs the Rust parity test without needing a Swift toolchain.
//! `xtask bench-large [GiB]` builds and runs the large-file benchmark in
//! release.

mod bench;

use std::collections::BTreeMap;
use std::fs;
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
    verify_fixture_copies(&repository)?;
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

/// Byte contents of every CSV fixture below `root`, keyed by `Folder/file`.
fn fixture_snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut files = BTreeMap::new();
    for folder in ["Heuristic", "Performance", "Reading"] {
        let directory = root.join(folder);
        let entries = fs::read_dir(&directory).map_err(|error| {
            format!(
                "cannot read fixture directory {}: {error}",
                directory.display()
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("cannot read fixture entry: {error}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "csv") {
                let name = format!("{folder}/{}", entry.file_name().to_string_lossy().as_ref());
                let bytes = fs::read(&path)
                    .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
                files.insert(name, bytes);
            }
        }
    }
    Ok(files)
}

/// Fails when the committed fixture copies are not byte-identical to the Apple
/// test bundle, so the manifest can never validate a corrupted copy.
fn verify_fixture_copies(repository: &Path) -> Result<(), String> {
    let apple = repository.join("app/apple/TabToolKit/Tests/TabToolKitTests/Fixtures");
    let copies = repository.join("app/universal/testdata/fixtures");
    if !apple.is_dir() {
        return Err(format!(
            "Apple fixture source is missing: {}",
            apple.display()
        ));
    }

    let apple_files = fixture_snapshot(&apple)?;
    let copy_files = fixture_snapshot(&copies)?;
    for (name, bytes) in &apple_files {
        match copy_files.get(name) {
            None => return Err(format!("fixture copy is missing: {name}")),
            Some(copy) if copy != bytes => {
                return Err(format!("fixture copy differs from Apple: {name}"));
            }
            Some(_) => {}
        }
    }
    for name in copy_files.keys() {
        if !apple_files.contains_key(name) {
            return Err(format!("fixture copy has no Apple source: {name}"));
        }
    }
    Ok(())
}

/// Runs the Rust parity test against the committed golden files.
fn check() -> Result<(), String> {
    if std::env::consts::OS != "macos" {
        return Err(
            "xtask golden --check requires macOS: the golden files pin Foundation behavior"
                .to_owned(),
        );
    }
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
        usage();
        return ExitCode::from(2);
    };

    let result = match command.as_str() {
        "golden" => match arguments.next().as_deref() {
            None => regenerate(),
            Some("--check") => check(),
            _ => {
                usage();
                return ExitCode::from(2);
            }
        },
        "bench-large" => {
            let size = arguments.next();
            if arguments.next().is_some() {
                usage();
                return ExitCode::from(2);
            }
            bench::bench_large(size.as_deref())
        }
        _ => {
            usage();
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

fn usage() {
    eprintln!("usage: xtask golden [--check]");
    eprintln!("       xtask bench-large [GiB]");
}

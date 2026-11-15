//! Atomic small-file I/O used by document saves.

use std::fs;
use std::io;

use tabtool_document::{StdStorage, Storage};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().expect("a scratch directory")
}

#[test]
fn read_returns_the_file_bytes() {
    let directory = scratch();
    let path = directory.path().join("document.csv");
    fs::write(&path, "a,b\n1,2\n").expect("a readable file");

    assert_eq!(StdStorage.read(&path).expect("the bytes"), b"a,b\n1,2\n");
}

#[test]
fn read_reports_a_missing_file() {
    let directory = scratch();
    let path = directory.path().join("missing.csv");

    let error = StdStorage.read(&path).expect_err("a missing file fails");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn write_atomic_creates_a_new_file() {
    let directory = scratch();
    let path = directory.path().join("document.csv");

    StdStorage
        .write_atomic(&path, b"a,b\n")
        .expect("a writable directory");

    assert_eq!(fs::read(&path).expect("the saved file"), b"a,b\n");
}

#[test]
fn write_atomic_replaces_the_whole_file() {
    let directory = scratch();
    let path = directory.path().join("document.csv");
    fs::write(&path, "old content that is longer\n").expect("a readable file");

    StdStorage
        .write_atomic(&path, b"new\n")
        .expect("an existing target is replaced");

    assert_eq!(fs::read(&path).expect("the saved file"), b"new\n");
}

#[test]
fn write_atomic_leaves_no_temporary_file_behind() {
    let directory = scratch();
    let path = directory.path().join("document.csv");

    StdStorage
        .write_atomic(&path, b"a\n")
        .expect("a writable directory");
    StdStorage
        .write_atomic(&path, b"b\n")
        .expect("a replaceable target");

    let names: Vec<_> = fs::read_dir(directory.path())
        .expect("the directory")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    assert_eq!(names, vec![path.file_name().expect("a file name")]);
}

#[cfg(unix)]
#[test]
fn write_atomic_preserves_an_existing_files_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = scratch();
    let path = directory.path().join("document.csv");
    fs::write(&path, "old\n").expect("a readable file");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("chmod");

    StdStorage
        .write_atomic(&path, b"new\n")
        .expect("an existing target is replaced");

    let mode = fs::metadata(&path)
        .expect("the saved file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o640);
}

#[cfg(unix)]
#[test]
fn write_atomic_gives_a_new_file_a_readable_mode() {
    use std::os::unix::fs::PermissionsExt;

    let directory = scratch();
    let path = directory.path().join("document.csv");

    StdStorage
        .write_atomic(&path, b"a\n")
        .expect("a writable directory");

    let mode = fs::metadata(&path)
        .expect("the saved file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
}

#[test]
fn write_atomic_reports_a_missing_directory() {
    let directory = scratch();
    let path = directory.path().join("missing").join("document.csv");

    let error = StdStorage
        .write_atomic(&path, b"a\n")
        .expect_err("the directory does not exist");

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!path.exists());
}

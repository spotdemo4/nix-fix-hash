use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fix-hash-test-{}-{sequence}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&source, &destination);
        } else {
            fs::copy(source, destination).unwrap();
        }
    }
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "fix-hash exited with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires nix and network access"]
fn fixes_bad_nix() {
    let root = env!("CARGO_MANIFEST_DIR");
    let bad = fs::read_to_string(format!("{root}/tests/bad.nix")).unwrap();
    let good = fs::read_to_string(format!("{root}/tests/good.nix")).unwrap();

    let tmp = TempDir::new("fetchers");
    let bad_path = tmp.0.join("bad.nix");
    fs::write(&bad_path, &bad).unwrap();

    let bin = env!("CARGO_BIN_EXE_fix-hash");
    let output = Command::new(bin)
        .current_dir(&tmp.0)
        .args(["--file", "bad.nix"])
        .output()
        .expect("failed to spawn fix-hash");
    assert_success(&output);

    let result = fs::read_to_string(&bad_path).unwrap();
    assert_eq!(result, good);
}

#[test]
#[ignore = "requires nix"]
fn fixes_cascading_hashes_in_one_invocation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cascading");
    let bad = fs::read_to_string(root.join("bad.nix")).unwrap();
    let good = fs::read_to_string(root.join("good.nix")).unwrap();

    let tmp = TempDir::new("cascading");
    let package_path = tmp.0.join("package.nix");
    fs::write(&package_path, bad).unwrap();
    copy_dir(&root.join("crate"), &tmp.0.join("crate"));

    let bin = env!("CARGO_BIN_EXE_fix-hash");
    let output = Command::new(bin)
        .current_dir(&tmp.0)
        .args(["--check", "--file", "package.nix"])
        .output()
        .expect("failed to spawn fix-hash");
    assert_success(&output);

    let result = fs::read_to_string(&package_path).unwrap();
    assert_eq!(result, good);
}

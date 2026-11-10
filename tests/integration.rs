// Test setup helpers panic when the filesystem fails.
#![allow(clippy::unwrap_used)]

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

fn run(program: &str, args: &[&str], nix_config: Option<&str>) -> String {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(nix_config) = nix_config {
        command.env("NIX_CONFIG", nix_config);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{program} {} exited with {}\nstderr:\n{}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
#[ignore = "requires nix as a trusted user"]
fn fixes_hash_when_stale_output_is_substitutable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/substituted");
    let bad = fs::read_to_string(root.join("bad.nix")).unwrap();
    let good = fs::read_to_string(root.join("good.nix")).unwrap();

    let tmp = TempDir::new("substituted");
    let package_path = tmp.0.join("package.nix");
    fs::write(&package_path, bad).unwrap();
    let package = package_path.to_str().unwrap();

    let secret_key = tmp.0.join("secret-key");
    let secret = run(
        "nix",
        &[
            "--extra-experimental-features",
            "nix-command",
            "key",
            "generate-secret",
            "--key-name",
            "fix-hash-test",
        ],
        None,
    );
    fs::write(&secret_key, &secret).unwrap();
    let public = Command::new("nix")
        .args([
            "--extra-experimental-features",
            "nix-command",
            "key",
            "convert-secret-to-public",
        ])
        .stdin(fs::File::open(&secret_key).unwrap())
        .output()
        .unwrap();
    assert!(public.status.success());
    let public = String::from_utf8_lossy(&public.stdout).trim().to_owned();

    let cache = format!(
        "file://{}?secret-key={}",
        tmp.0.join("cache").display(),
        secret_key.display()
    );

    // add the output that matches the stale hash straight to a signed binary cache, since building
    // it in the local store could leave it alive, e.g. queued for upload by a post-build hook
    let stale_content = tmp.0.join("stale");
    fs::write(&stale_content, "old").unwrap();
    let stale_out = run(
        "nix",
        &[
            "--extra-experimental-features",
            "nix-command",
            "store",
            "add",
            "--mode",
            "flat",
            "--name",
            "fix-hash-substituted",
            "--store",
            &cache,
            stale_content.to_str().unwrap(),
        ],
        None,
    );

    let nix_config = format!(
        "extra-substituters = file://{}\nextra-trusted-public-keys = {public}",
        tmp.0.join("cache").display()
    );

    // the stale output must be substitutable for the changed derivation, or the test proves nothing
    let changed_drv = run("nix-instantiate", &[package], None);
    let substituted = run("nix-store", &["--realise", &changed_drv], Some(&nix_config));
    assert_eq!(substituted, stale_out);
    run("nix-store", &["--delete", &stale_out], None);

    let bin = env!("CARGO_BIN_EXE_fix-hash");
    let output = Command::new(bin)
        .current_dir(&tmp.0)
        .env("NIX_CONFIG", &nix_config)
        .args(["--file", "package.nix"])
        .output()
        .expect("failed to spawn fix-hash");
    assert_success(&output);

    let result = fs::read_to_string(&package_path).unwrap();
    assert_eq!(result, good);
}

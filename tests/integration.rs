use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("fix-hash-test-{}", std::process::id()));
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

#[test]
#[ignore = "requires nix and network access"]
fn fixes_bad_nix() {
    let root = env!("CARGO_MANIFEST_DIR");
    let bad = fs::read_to_string(format!("{root}/tests/bad.nix")).unwrap();
    let good = fs::read_to_string(format!("{root}/tests/good.nix")).unwrap();

    let tmp = TempDir::new();
    let bad_path = tmp.0.join("bad.nix");
    fs::write(&bad_path, &bad).unwrap();

    let bin = env!("CARGO_BIN_EXE_fix-hash");
    let status = Command::new(bin)
        .current_dir(&tmp.0)
        .args(["--file", "bad.nix"])
        .status()
        .expect("failed to spawn fix-hash");
    assert!(status.success(), "fix-hash exited with {status}");

    let result = fs::read_to_string(&bad_path).unwrap();
    assert_eq!(result, good);
}

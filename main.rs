use colored::Colorize;
use ignore::WalkBuilder;
use ignore::WalkState;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::mpsc;

type BoxError = Box<dyn Error + Send + Sync>;

fn step(label: &str, msg: impl Display) {
    println!("{: >12} {msg}", label.blue().bold());
}

struct Derivation {
    path: String,
    hash: String,
}

struct NixFile {
    path: PathBuf,
    content: String,
}

// Collects all nix files in the given directory that contain any of the given hashes
fn collect_nix_files(dir: &Path, hashes: &HashSet<String>) -> Vec<NixFile> {
    let hashes = Arc::new(hashes.clone());
    let (tx, rx) = mpsc::channel::<NixFile>();

    WalkBuilder::new(dir).build_parallel().run(|| {
        let tx = tx.clone();
        let hashes = Arc::clone(&hashes);
        Box::new(move |result| {
            let Ok(entry) = result else {
                return WalkState::Continue;
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return WalkState::Continue;
            }
            let path = entry.into_path();
            if path.extension().is_none_or(|e| e != "nix") {
                return WalkState::Continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                return WalkState::Continue;
            };
            if hashes.iter().any(|h| content.contains(h)) {
                let _ = tx.send(NixFile { path, content });
            }
            WalkState::Continue
        })
    });
    drop(tx);

    rx.iter().collect()
}

// Builds an index mapping each hash to the list of files that contain it
fn build_index(files: &[NixFile], hashes: &HashSet<String>) -> HashMap<String, Vec<usize>> {
    let mut idx: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, file) in files.iter().enumerate() {
        for hash in hashes {
            if file.content.contains(hash) {
                idx.entry(hash.clone()).or_default().push(i);
            }
        }
    }
    idx
}

// Gets the fixed-output derivations for the given arguments, returning their paths and hashes
fn fixed_output_derivations(args: &[String]) -> Result<Vec<Derivation>, BoxError> {
    let output = Command::new("nix")
        .args([
            "derivation",
            "show",
            "--extra-experimental-features",
            "nix-command",
            "--extra-experimental-features",
            "flakes",
            "--recursive",
        ])
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let Some(drvs) = json.get("derivations").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let mut by_hash: IndexMap<String, Derivation> = IndexMap::new();
    for (key, drv) in drvs {
        let Some(hash) = drv["outputs"]["out"]["hash"].as_str() else {
            continue;
        };
        by_hash
            .entry(hash.to_owned())
            .or_insert_with(|| Derivation {
                path: format!("/nix/store/{key}"),
                hash: hash.to_owned(),
            });
    }
    Ok(by_hash.into_values().collect())
}

// Checks if the output of a given derivation already exists in the nix store
fn exists(path: &str) -> Result<bool, BoxError> {
    let output = Command::new("nix-store")
        .args(["--query", "--hash", "--use-output"])
        .arg(path)
        .output()?;

    Ok(output.status.success())
}

static HASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sha256-[A-Za-z0-9+/]{43}=?").unwrap());

// Realizes the given derivation, returning the new hash if it differs from the existing one
fn realise(path: &str, hash: &str) -> Result<Option<String>, BoxError> {
    let mut cmd = Command::new("nix-store");
    cmd.args(["--quiet", "--no-build-output", "--realise"]);

    // Rebuilds the derivation and checks whether the result is identical with the existing outputs
    if exists(path)? {
        cmd.arg("--check");
    }

    let output = cmd.arg(path).output()?;
    if output.status.success() {
        return Ok(None);
    }

    // Finds hashes in the error output
    let err = String::from_utf8_lossy(&output.stderr);
    let mut matches: HashSet<String> = HASH_RE
        .find_iter(err.trim())
        .map(|m| m.as_str().to_owned())
        .collect();
    if matches.is_empty() || !matches.remove(hash) {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    Ok(matches.into_iter().next())
}

fn build(args: &[String]) -> Result<(), BoxError> {
    let output = Command::new("nix")
        .args([
            "build",
            "--extra-experimental-features",
            "nix-command",
            "--extra-experimental-features",
            "flakes",
            "--no-warn-dirty",
            "--no-link",
        ])
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    Ok(())
}

fn main() -> Result<(), BoxError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir()?;

    step(
        "Parsing",
        format!("nix derivation show -r {}", args.join(" ")),
    );
    let derivations = fixed_output_derivations(&args)?;

    step("Collecting", cwd.join("*.nix").display());
    let hashes: HashSet<String> = derivations.iter().map(|d| d.hash.clone()).collect();
    let mut nix_files = collect_nix_files(&cwd, &hashes);
    let index = build_index(&nix_files, &hashes);

    let realised = std::thread::scope(|s| -> Result<Vec<(Derivation, String)>, BoxError> {
        let handles: Vec<_> = derivations
            .into_iter()
            .filter(|d| index.get(&d.hash).is_some_and(|v| !v.is_empty()))
            .map(|drv| {
                s.spawn(move || -> Result<Option<(Derivation, String)>, BoxError> {
                    step("Realizing", format!("nix-store --realise {}", drv.path));
                    let Some(next) = realise(&drv.path, &drv.hash)? else {
                        return Ok(None);
                    };

                    step("Realized", format!("{} -> {}", drv.hash, next));
                    Ok(Some((drv, next)))
                })
            })
            .collect();

        let mut out = Vec::new();
        for h in handles {
            if let Some(pair) = h.join().unwrap()? {
                out.push(pair);
            }
        }
        Ok(out)
    })?;

    for (drv, next_hash) in realised {
        for &i in &index[&drv.hash] {
            let file = &mut nix_files[i];
            step("Patching", file.path.display());
            file.content = file.content.replace(&drv.hash, &next_hash);
            std::fs::write(&file.path, &file.content)?;
        }
    }

    step("Building", format!("nix build {}", args.join(" ")));
    build(&args)?;

    Ok(())
}

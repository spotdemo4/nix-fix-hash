use colored::Colorize;
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

type BoxError = Box<dyn Error>;

fn files_with_hash(dir: &Path, fod_hash: &str) -> Vec<PathBuf> {
    WalkBuilder::new(dir)
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .map(ignore::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|ext| ext == "nix"))
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|s| s.contains(fod_hash)))
        .collect()
}

fn patch_hash(path: &Path, old_hash: &str, new_hash: &str) -> Result<(), BoxError> {
    let content = std::fs::read_to_string(path)?;
    let new_content = content.replace(old_hash, new_hash);
    std::fs::write(path, new_content)?;
    Ok(())
}

struct Derivation {
    path: String,
    hash: String,
}

fn fixed_output_derivations(args: &[String]) -> Result<Vec<Derivation>, BoxError> {
    let output = Command::new("nix")
        .args(["derivation", "show", "-r"])
        .args(args)
        .output()?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let Some(drvs) = json.get("derivations").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };

    let mut seen = HashSet::new();
    Ok(drvs
        .iter()
        .filter_map(|(key, drv)| {
            let path = format!("/nix/store/{key}");
            let hash = drv["outputs"]["out"]["hash"].as_str()?.to_owned();

            seen.insert(hash.clone())
                .then_some(Derivation { path, hash })
        })
        .collect())
}

static HASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sha256-[A-Za-z0-9+/]{43}=?").unwrap());

fn realise(path: &str, hash: &str) -> Result<Option<String>, BoxError> {
    let output = Command::new("nix-store")
        .args(["--quiet", "--no-build-output", "--realise"])
        .arg(path)
        .output()?;

    if output.status.success() {
        return Ok(None);
    }

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
        .args(["build", "--no-link"])
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

    println!(
        "{: >12} nix derivation show -r {}",
        "Parsing".green().bold(),
        args.join(" ")
    );
    let derivations = fixed_output_derivations(&args)?;

    for drv in derivations {
        let files = files_with_hash(&cwd, &drv.hash);
        if files.is_empty() {
            continue;
        }

        println!(
            "{: >12} nix-store --realise {}",
            "Realizing".green().bold(),
            drv.path
        );
        let Some(next_hash) = realise(&drv.path, &drv.hash)? else {
            continue;
        };

        for file in files {
            println!("{: >12} {}", "Patching".green().bold(), file.display());
            patch_hash(&file, &drv.hash, &next_hash)?;
        }
    }

    println!(
        "{: >12} nix build {}",
        "Building".green().bold(),
        args.join(" ")
    );
    build(&args)?;

    Ok(())
}

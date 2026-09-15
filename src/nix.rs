use crate::BoxError;
use regex::Regex;
use std::collections::HashSet;
use std::process::Command;
use std::sync::LazyLock;

pub(crate) static HASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"sha256-[A-Za-z0-9+/]{43}=?").unwrap());

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Derivation {
    pub(crate) path: String,
    pub(crate) expected_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HashMismatch {
    pub(crate) specified: String,
    pub(crate) got: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum RealisationOutcome {
    Valid,
    DirectMismatch { actual_hash: String },
    DependencyBlocked { mismatches: Vec<HashMismatch> },
    HardError { message: String },
}

// Gets the fixed-output derivations for the given arguments, returning their paths and hashes
pub(crate) fn fixed_output_derivations(args: &[String]) -> Result<Vec<Derivation>, BoxError> {
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

    let Some(drvs) = json
        .get("derivations")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(Vec::new());
    };

    let mut derivations = Vec::new();
    for (key, drv) in drvs {
        let Some(hash) = drv["outputs"]["out"]["hash"].as_str() else {
            continue;
        };
        derivations.push(Derivation {
            path: format!("/nix/store/{key}"),
            expected_hash: hash.to_owned(),
        });
    }
    derivations.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(derivations)
}

// Checks if the output of a given derivation already exists in the nix store
fn exists(path: &str) -> Result<bool, BoxError> {
    let output = Command::new("nix-store")
        .args(["--query", "--hash", "--use-output"])
        .arg(path)
        .output()?;

    Ok(output.status.success())
}

fn labeled_hash(line: &str, label: &str) -> Option<String> {
    let value = line.trim().strip_prefix(label)?.trim();
    let matched = HASH_RE.find(value)?;
    (matched.start() == 0 && matched.end() == value.len()).then(|| value.to_owned())
}

fn parse_hash_mismatches(stderr: &str) -> Vec<HashMismatch> {
    let mut specified = None;
    let mut mismatches = Vec::new();

    for line in stderr.lines() {
        if let Some(hash) = labeled_hash(line, "specified:") {
            specified = Some(hash);
            continue;
        }

        if let Some(got) = labeled_hash(line, "got:")
            && let Some(specified) = specified.take()
        {
            mismatches.push(HashMismatch { specified, got });
        }
    }

    mismatches
}

fn hard_error(message: impl Into<String>) -> RealisationOutcome {
    RealisationOutcome::HardError {
        message: message.into(),
    }
}

fn classify_realisation(succeeded: bool, expected_hash: &str, stderr: &str) -> RealisationOutcome {
    if succeeded {
        return RealisationOutcome::Valid;
    }

    let mismatches = parse_hash_mismatches(stderr);
    if mismatches.is_empty() {
        let message = stderr.trim();
        return hard_error(if message.is_empty() {
            "nix-store failed without a recognized hash mismatch"
        } else {
            message
        });
    }

    let actual_hashes: HashSet<&str> = mismatches
        .iter()
        .filter(|mismatch| mismatch.specified == expected_hash)
        .map(|mismatch| mismatch.got.as_str())
        .collect();

    if actual_hashes.is_empty() {
        return RealisationOutcome::DependencyBlocked { mismatches };
    }
    if actual_hashes.contains(expected_hash) {
        return hard_error(format!(
            "nix reported identical specified and actual hashes for {expected_hash}"
        ));
    }
    if actual_hashes.len() > 1 {
        let mut actual_hashes: Vec<_> = actual_hashes.into_iter().collect();
        actual_hashes.sort_unstable();
        return hard_error(format!(
            "nix reported conflicting actual hashes for {expected_hash}: {}",
            actual_hashes.join(", ")
        ));
    }

    RealisationOutcome::DirectMismatch {
        actual_hash: (*actual_hashes.iter().next().unwrap()).to_owned(),
    }
}

// Realizes the given derivation and classifies any hash mismatch it reports
pub(crate) fn realise(derivation: &Derivation) -> RealisationOutcome {
    let mut cmd = Command::new("nix-store");
    cmd.args(["--quiet", "--no-build-output", "--realise"]);

    match exists(&derivation.path) {
        Ok(true) => {
            // Rebuilds the derivation and checks whether the result is identical with the existing outputs
            cmd.arg("--check");
        }
        Ok(false) => {}
        Err(error) => {
            return hard_error(format!(
                "failed to query {} before realization: {error}",
                derivation.path
            ));
        }
    }

    match cmd.arg(&derivation.path).output() {
        Ok(output) => classify_realisation(
            output.status.success(),
            &derivation.expected_hash,
            &String::from_utf8_lossy(&output.stderr),
        ),
        Err(error) => hard_error(format!("failed to realize {}: {error}", derivation.path)),
    }
}

pub(crate) fn build(args: &[String]) -> Result<(), BoxError> {
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

#[cfg(test)]
mod tests {
    use super::{HashMismatch, RealisationOutcome, classify_realisation, parse_hash_mismatches};

    const A: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const B: &str = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";
    const C: &str = "sha256-CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC=";

    #[test]
    fn parses_labeled_hash_mismatches() {
        let stderr = format!(
            "unrelated {C}\n  specified: {A}\n     got:     {B}\n\nspecified: {B}\ngot: {C}"
        );

        assert_eq!(
            parse_hash_mismatches(&stderr),
            [
                HashMismatch {
                    specified: A.to_owned(),
                    got: B.to_owned(),
                },
                HashMismatch {
                    specified: B.to_owned(),
                    got: C.to_owned(),
                },
            ]
        );
    }

    #[test]
    fn ignores_incomplete_hash_mismatches() {
        let stderr = format!("specified: {A}\nwarning: stopped early");

        assert!(parse_hash_mismatches(&stderr).is_empty());
    }

    #[test]
    fn classifies_successful_realisation_as_valid() {
        assert_eq!(classify_realisation(true, A, ""), RealisationOutcome::Valid);
    }

    #[test]
    fn classifies_matching_hash_mismatch_as_direct() {
        let stderr = format!("specified: {A}\ngot: {B}");

        assert_eq!(
            classify_realisation(false, A, &stderr),
            RealisationOutcome::DirectMismatch {
                actual_hash: B.to_owned(),
            }
        );
    }

    #[test]
    fn classifies_other_hash_mismatch_as_dependency_blocked() {
        let stderr = format!("specified: {A}\ngot: {B}");

        assert_eq!(
            classify_realisation(false, C, &stderr),
            RealisationOutcome::DependencyBlocked {
                mismatches: vec![HashMismatch {
                    specified: A.to_owned(),
                    got: B.to_owned(),
                }],
            }
        );
    }

    #[test]
    fn classifies_unrecognized_failure_as_hard_error() {
        assert_eq!(
            classify_realisation(false, A, "builder failed"),
            RealisationOutcome::HardError {
                message: "builder failed".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_conflicting_direct_hashes() {
        let stderr = format!("specified: {A}\ngot: {B}\nspecified: {A}\ngot: {C}");

        assert!(matches!(
            classify_realisation(false, A, &stderr),
            RealisationOutcome::HardError { .. }
        ));
    }
}

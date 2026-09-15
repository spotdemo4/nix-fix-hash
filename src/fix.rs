use crate::nix::{
    Derivation, HASH_RE, HashMismatch, RealisationOutcome, fixed_output_derivations, realise,
};
use crate::{BoxError, step};
use ignore::{WalkBuilder, WalkState};
use indexmap::IndexMap;
use regex::Captures;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

const MAX_REALISATION_WORKERS: usize = 8;
const MAX_UPDATE_ROUNDS: usize = 32;

#[derive(Debug)]
struct NixFile {
    path: PathBuf,
    content: String,
}

#[derive(Debug)]
struct Realisation {
    derivation: Derivation,
    outcome: RealisationOutcome,
}

#[derive(Debug)]
struct RoundPlan {
    replacements: IndexMap<String, String>,
    blocked: Vec<String>,
}

struct FilePatch {
    index: usize,
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
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                return WalkState::Continue;
            }
            let path = entry.into_path();
            if path.extension().is_none_or(|extension| extension != "nix") {
                return WalkState::Continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                return WalkState::Continue;
            };
            if hashes.iter().any(|hash| content.contains(hash)) {
                let _ = tx.send(NixFile { path, content });
            }
            WalkState::Continue
        })
    });
    drop(tx);

    let mut files: Vec<_> = rx.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

// Builds an index mapping each hash to the list of files that contain it
fn build_index(files: &[NixFile], hashes: &HashSet<String>) -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (file_index, file) in files.iter().enumerate() {
        for hash in hashes {
            if file.content.contains(hash) {
                index.entry(hash.clone()).or_default().push(file_index);
            }
        }
    }
    index
}

fn realise_all(derivations: Vec<Derivation>) -> Vec<Realisation> {
    let mut pending = derivations.into_iter();
    let mut results = Vec::new();

    loop {
        let batch: Vec<_> = pending.by_ref().take(MAX_REALISATION_WORKERS).collect();
        if batch.is_empty() {
            break;
        }

        let mut batch_results = std::thread::scope(|scope| {
            let handles: Vec<_> = batch
                .into_iter()
                .map(|derivation| {
                    step(
                        "Realizing",
                        format!("nix-store --realise {}", derivation.path),
                    );
                    let panic_derivation = derivation.clone();
                    let handle = scope.spawn(move || {
                        let outcome = realise(&derivation);
                        Realisation {
                            derivation,
                            outcome,
                        }
                    });
                    (panic_derivation, handle)
                })
                .collect();

            handles
                .into_iter()
                .map(|(derivation, handle)| match handle.join() {
                    Ok(result) => result,
                    Err(_) => Realisation {
                        derivation,
                        outcome: RealisationOutcome::HardError {
                            message: "realization worker panicked".to_owned(),
                        },
                    },
                })
                .collect::<Vec<_>>()
        });
        results.append(&mut batch_results);
    }

    results.sort_by(|a, b| a.derivation.path.cmp(&b.derivation.path));
    for result in &results {
        if let RealisationOutcome::DirectMismatch { actual_hash } = &result.outcome {
            step(
                "Realized",
                format!("{} -> {actual_hash}", result.derivation.expected_hash),
            );
        }
    }
    results
}

fn format_mismatches(mismatches: &[HashMismatch]) -> String {
    mismatches
        .iter()
        .map(|mismatch| format!("{} -> {}", mismatch.specified, mismatch.got))
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_outcome(outcome: &RealisationOutcome) -> String {
    match outcome {
        RealisationOutcome::Valid => "valid".to_owned(),
        RealisationOutcome::DirectMismatch { actual_hash } => {
            format!("direct mismatch -> {actual_hash}")
        }
        RealisationOutcome::DependencyBlocked { mismatches } => {
            format!("blocked by {}", format_mismatches(mismatches))
        }
        RealisationOutcome::HardError { message } => format!("failed: {message}"),
    }
}

fn plan_round(results: &[Realisation]) -> Result<RoundPlan, BoxError> {
    let hard_errors: Vec<_> = results
        .iter()
        .filter_map(|result| match &result.outcome {
            RealisationOutcome::HardError { message } => {
                Some(format!("{}: {message}", result.derivation.path))
            }
            _ => None,
        })
        .collect();
    if !hard_errors.is_empty() {
        return Err(format!(
            "failed to realize fixed-output derivations:\n{}",
            hard_errors.join("\n")
        )
        .into());
    }

    let blocked = results
        .iter()
        .filter_map(|result| match &result.outcome {
            RealisationOutcome::DependencyBlocked { mismatches } => Some(format!(
                "{}: {}",
                result.derivation.path,
                format_mismatches(mismatches)
            )),
            _ => None,
        })
        .collect();

    let mut by_hash: IndexMap<String, Vec<&Realisation>> = IndexMap::new();
    for result in results {
        by_hash
            .entry(result.derivation.expected_hash.clone())
            .or_default()
            .push(result);
    }

    let mut replacements = IndexMap::new();
    for (expected_hash, group) in by_hash {
        let direct: Vec<_> = group
            .iter()
            .filter_map(|result| match &result.outcome {
                RealisationOutcome::DirectMismatch { actual_hash } => Some(actual_hash),
                _ => None,
            })
            .collect();
        if direct.is_empty() {
            continue;
        }

        if group.len() > 1 {
            let details = group
                .iter()
                .map(|result| {
                    format!(
                        "{} ({})",
                        result.derivation.path,
                        describe_outcome(&result.outcome)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "cannot safely replace shared hash {expected_hash}; used by {details}"
            )
            .into());
        }

        let actual_hash = direct[0];
        replacements.insert(expected_hash, actual_hash.clone());
    }

    Ok(RoundPlan {
        replacements,
        blocked,
    })
}

fn replace_hashes(content: &str, replacements: &IndexMap<String, String>) -> (String, usize) {
    let mut replaced = 0;
    let content = HASH_RE
        .replace_all(content, |captures: &Captures<'_>| {
            replacements.get(&captures[0]).map_or_else(
                || captures[0].to_owned(),
                |replacement| {
                    replaced += 1;
                    replacement.clone()
                },
            )
        })
        .into_owned();
    (content, replaced)
}

fn plan_file_patches(files: &[NixFile], replacements: &IndexMap<String, String>) -> Vec<FilePatch> {
    files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let (content, replaced) = replace_hashes(&file.content, replacements);
            (replaced > 0 && content != file.content).then_some(FilePatch { index, content })
        })
        .collect()
}

fn validate_replacement_occurrences(
    files: &[NixFile],
    replacements: &IndexMap<String, String>,
) -> Result<(), BoxError> {
    for expected_hash in replacements.keys() {
        let mut occurrences = 0;
        let mut locations = Vec::new();
        for file in files {
            let file_occurrences = file.content.matches(expected_hash).count();
            if file_occurrences > 0 {
                occurrences += file_occurrences;
                locations.push(format!("{} ({file_occurrences})", file.path.display()));
            }
        }

        if occurrences != 1 {
            return Err(format!(
                "cannot safely replace hash {expected_hash}; found {occurrences} textual occurrences in {}",
                locations.join(", ")
            )
            .into());
        }
    }

    Ok(())
}

fn apply_replacements(
    files: &mut [NixFile],
    replacements: &IndexMap<String, String>,
) -> Result<usize, BoxError> {
    let patches = plan_file_patches(files, replacements);
    let changed_files = patches.len();

    for patch in patches {
        let file = &mut files[patch.index];
        std::fs::write(&file.path, &patch.content)?;
        file.content = patch.content;
        step("Patching", file.path.display());
    }

    Ok(changed_files)
}

pub(crate) fn fix_hashes(cwd: &Path, args: &[String]) -> Result<(), BoxError> {
    let mut seen_states: HashMap<Vec<(String, String)>, usize> = HashMap::new();
    let mut round = 0;
    let mut update_rounds = 0;

    loop {
        round += 1;
        step("Round", round);
        step(
            "Parsing",
            format!("nix derivation show -r {}", args.join(" ")),
        );
        let derivations = fixed_output_derivations(args)?;

        step("Collecting", cwd.display());
        let hashes: HashSet<String> = derivations
            .iter()
            .map(|derivation| derivation.expected_hash.clone())
            .collect();
        let mut nix_files = collect_nix_files(cwd, &hashes);
        let index = build_index(&nix_files, &hashes);
        let relevant: Vec<_> = derivations
            .into_iter()
            .filter(|derivation| {
                index
                    .get(&derivation.expected_hash)
                    .is_some_and(|files| !files.is_empty())
            })
            .collect();

        let state: Vec<_> = relevant
            .iter()
            .map(|derivation| (derivation.path.clone(), derivation.expected_hash.clone()))
            .collect();
        if let Some(first_round) = seen_states.insert(state, round) {
            return Err(format!(
                "hash fixing did not converge: derivation state from round {first_round} repeated in round {round}"
            )
            .into());
        }

        let results = realise_all(relevant);
        let plan = plan_round(&results)?;
        if !plan.replacements.is_empty() {
            validate_replacement_occurrences(&nix_files, &plan.replacements)?;
            if update_rounds == MAX_UPDATE_ROUNDS {
                return Err(format!(
                    "hash fixing did not converge after {MAX_UPDATE_ROUNDS} update rounds"
                )
                .into());
            }

            let changed_files = apply_replacements(&mut nix_files, &plan.replacements)?;
            if changed_files == 0 {
                return Err(
                    "hash fixing could not make progress: replacements changed no files".into(),
                );
            }
            update_rounds += 1;
            continue;
        }

        if !plan.blocked.is_empty() {
            return Err(format!(
                "hash fixing could not make progress; dependency hash mismatches remain:\n{}",
                plan.blocked.join("\n")
            )
            .into());
        }

        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NixFile, Realisation, plan_file_patches, plan_round, replace_hashes,
        validate_replacement_occurrences,
    };
    use crate::nix::{Derivation, HashMismatch, RealisationOutcome};
    use indexmap::IndexMap;
    use std::path::PathBuf;

    const A: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    const B: &str = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";
    const C: &str = "sha256-CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC=";

    fn derivation(path: &str, expected_hash: &str) -> Derivation {
        Derivation {
            path: path.to_owned(),
            expected_hash: expected_hash.to_owned(),
        }
    }

    #[test]
    fn plans_direct_updates_while_dependencies_are_blocked() {
        let results = [
            Realisation {
                derivation: derivation("/nix/store/source.drv", A),
                outcome: RealisationOutcome::DirectMismatch {
                    actual_hash: B.to_owned(),
                },
            },
            Realisation {
                derivation: derivation("/nix/store/cargo.drv", C),
                outcome: RealisationOutcome::DependencyBlocked {
                    mismatches: vec![HashMismatch {
                        specified: A.to_owned(),
                        got: B.to_owned(),
                    }],
                },
            },
        ];

        let plan = plan_round(&results).unwrap();

        assert_eq!(plan.replacements[A], B);
        assert_eq!(plan.blocked.len(), 1);
    }

    #[test]
    fn records_blocked_round_without_replacements() {
        let results = [Realisation {
            derivation: derivation("/nix/store/cargo.drv", C),
            outcome: RealisationOutcome::DependencyBlocked {
                mismatches: vec![HashMismatch {
                    specified: A.to_owned(),
                    got: B.to_owned(),
                }],
            },
        }];

        let plan = plan_round(&results).unwrap();

        assert!(plan.replacements.is_empty());
        assert_eq!(plan.blocked.len(), 1);
    }

    #[test]
    fn rejects_shared_hash_updates() {
        let results = [
            Realisation {
                derivation: derivation("/nix/store/one.drv", A),
                outcome: RealisationOutcome::DirectMismatch {
                    actual_hash: B.to_owned(),
                },
            },
            Realisation {
                derivation: derivation("/nix/store/two.drv", A),
                outcome: RealisationOutcome::DirectMismatch {
                    actual_hash: B.to_owned(),
                },
            },
        ];

        let error = plan_round(&results).unwrap_err().to_string();

        assert!(error.contains("cannot safely replace shared hash"));
        assert!(error.contains("one.drv"));
        assert!(error.contains("two.drv"));
    }

    #[test]
    fn aggregates_hard_errors() {
        let results = [
            Realisation {
                derivation: derivation("/nix/store/one.drv", A),
                outcome: RealisationOutcome::HardError {
                    message: "first".to_owned(),
                },
            },
            Realisation {
                derivation: derivation("/nix/store/two.drv", B),
                outcome: RealisationOutcome::HardError {
                    message: "second".to_owned(),
                },
            },
        ];

        let error = plan_round(&results).unwrap_err().to_string();

        assert!(error.contains("one.drv: first"));
        assert!(error.contains("two.drv: second"));
    }

    #[test]
    fn applies_replacements_simultaneously() {
        let replacements =
            IndexMap::from([(A.to_owned(), B.to_owned()), (B.to_owned(), C.to_owned())]);

        let (content, replaced) = replace_hashes(&format!("{A} {B}"), &replacements);

        assert_eq!(content, format!("{B} {C}"));
        assert_eq!(replaced, 2);
    }

    #[test]
    fn rejects_replacements_with_multiple_textual_occurrences() {
        let files = [
            NixFile {
                path: PathBuf::from("requested.nix"),
                content: A.to_owned(),
            },
            NixFile {
                path: PathBuf::from("unrelated.nix"),
                content: A.to_owned(),
            },
        ];
        let replacements = IndexMap::from([(A.to_owned(), B.to_owned())]);

        let error = validate_replacement_occurrences(&files, &replacements)
            .unwrap_err()
            .to_string();

        assert!(error.contains("found 2 textual occurrences"));
        assert!(error.contains("requested.nix"));
        assert!(error.contains("unrelated.nix"));
    }

    #[test]
    fn plans_one_patch_for_multiple_replacements_in_a_file() {
        let files = [NixFile {
            path: PathBuf::from("package.nix"),
            content: format!("{A} {B}"),
        }];
        let replacements =
            IndexMap::from([(A.to_owned(), B.to_owned()), (B.to_owned(), C.to_owned())]);

        let patches = plan_file_patches(&files, &replacements);

        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].content, format!("{B} {C}"));
    }
}

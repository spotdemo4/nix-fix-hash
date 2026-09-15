mod fix;
mod nix;

use colored::Colorize;
use std::error::Error;
use std::fmt::Display;

pub(crate) type BoxError = Box<dyn Error + Send + Sync>;

pub(crate) fn step(label: &str, msg: impl Display) {
    println!("{: >12} {msg}", label.blue().bold());
}

struct Args {
    check: bool,
    nix: Vec<String>,
}

fn parse_args<I>(args: I) -> Args
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut check = false;
    let mut nix = Vec::new();

    for arg in args {
        for part in arg.as_ref().split_whitespace() {
            if part == "--check" {
                check = true;
            } else {
                nix.push(part.to_owned());
            }
        }
    }

    Args { check, nix }
}

fn main() -> Result<(), BoxError> {
    let args = parse_args(std::env::args().skip(1));
    let cwd = std::env::current_dir()?;

    fix::fix_hashes(&cwd, &args.nix)?;

    if args.check {
        step("Building", format!("nix build {}", args.nix.join(" ")));
        nix::build(&args.nix)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn parse_args_consumes_check_flag() {
        let args = parse_args(["--check", ".#pkg"]);

        assert!(args.check);
        assert_eq!(args.nix, [".#pkg"]);
    }

    #[test]
    fn parse_args_splits_action_arguments() {
        let args = parse_args(["--check --file package.nix"]);

        assert!(args.check);
        assert_eq!(args.nix, ["--file", "package.nix"]);
    }
}

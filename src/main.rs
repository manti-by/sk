//! `sk` manages agent skills the way `uv` manages Python packages: declare the
//! skills you want in `sk.toml`, then `sk sync` installs them under the skills
//! directory of the agent named by `project.agent`.

mod error;
mod github;
mod init;
mod lock;
mod manifest;
mod parallel;
mod paths;
mod sync;
mod util;

use std::path::PathBuf;
use std::process::ExitCode;

use error::{Error, Result};
use paths::Layout;

const USAGE: &str = "\
sk - manage agent skills from sk.toml

USAGE:
    sk <COMMAND> [OPTIONS] [DIR]

COMMANDS:
    init            Create the agent's skills directory, sk.toml and sk.lock
                    if they are missing
    sync            Download every skill in sk.toml into the agent's skills
                    directory (.agents/skills/, .claude/skills/ or
                    .opencode/skills/, picked by project.agent in sk.toml)

OPTIONS:
    sync --upgrade  Re-resolve branches and tags instead of using sk.lock commits
    -h, --help      Print this help
    -V, --version   Print the version

ENVIRONMENT:
    GITHUB_TOKEN    Token used for GitHub API requests, raising the rate limit.
                    GH_TOKEN is also accepted. Without one, sk is anonymous.
";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Parses arguments and dispatches to a command.
fn run(args: Vec<String>) -> Result<()> {
    let mut positional: Vec<String> = Vec::new();
    let mut upgrade = false;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" | "help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("sk {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            // `--upgrade` only means anything for `sync`.
            "--upgrade" => upgrade = true,
            other if other.starts_with('-') => {
                bail!("unknown option `{other}`; run `sk --help`")
            }
            _ => positional.push(arg),
        }
    }

    let Some((command, rest)) = positional.split_first() else {
        print!("{USAGE}");
        return Ok(());
    };
    let command = command.as_str();

    // Both commands take an optional project directory, defaulting to `.`.
    if let Some(extra) = rest.get(1) {
        bail!("unexpected argument `{extra}`; run `sk --help`");
    }
    let layout = Layout::new(
        rest.first()
            .map_or_else(|| PathBuf::from("."), PathBuf::from),
    );

    match command {
        "init" => {
            if upgrade {
                bail!("`--upgrade` is only valid for `sk sync`");
            }
            init::run(&layout)
        }
        "sync" => sync::run(&layout, upgrade),
        other => Err(Error::new(format!(
            "unknown command `{other}`; expected `init` or `sync`"
        ))),
    }
}
